#!/usr/bin/env python3
"""LLM-powered move advisor for MTGA.

The decklist comes from the match data in Player.log (the deck the client
submits at match start). The strategy comes from a plain-text file the deck
designer wrote in arena/strategies/.
"""

import argparse
import json
import os
import sys
from collections import Counter
from pathlib import Path

import anthropic
from pydantic import BaseModel

try:
    from mtga_state import CardDatabase, GameState
    from parse_game import parse_log
except ImportError:  # pragma: no cover - useful when imported as tools.advisor
    from arena.mtga_state import CardDatabase, GameState
    from arena.parse_game import parse_log


DEFAULT_MODEL = "claude-opus-5-5"
DEFAULT_EFFORT = "low"
MAX_TOKENS = 16000
REQUEST_TIMEOUT_SECONDS = 30.0
FALLBACK_BETA = "server-side-fallback-2026-07-01"


def advisor_model():
    return os.environ.get("ARENA_ADVISOR_MODEL", DEFAULT_MODEL)


def advisor_effort():
    return os.environ.get("ARENA_ADVISOR_EFFORT", DEFAULT_EFFORT)

STRATEGIES_DIR = Path(__file__).resolve().parent / "strategies"

# Role hints for the offline heuristic only. The LLM path gets its deck from
# the match log and its strategy from the designer's strategy file.
ENABLERS = {
    "Does Machines",
    "Overlord of the Balemurk",
    "Dark Matter Manipulator",
    "Theorix Charm",
}
REANIMATORS = {"Superior Spider-Man"}
REMOVAL = {"Bitter Triumph", "Grim Bauble"}
BIG_TARGETS = {
    "Bringer of the Last Gift",
    "Valgavoth, Terror Eater",
    "Terror of the Peaks",
    "Marang River Regent",
    "Deceit",
    "Ardyn, the Usurper",
}
LANDS = {
    "Cavern of Souls",
    "Watery Grave",
    "Gloomlake Verge",
    "Multiversal Passage",
    "Starting Town",
    "Undercity Sewers",
    "Pit of Offerings",
    "Swamp",
    "Island",
}


def deck_from_state(game_state):
    """Return the decklist read from the match log, or {}."""
    if isinstance(game_state, dict):
        deck = game_state.get("deck")
        if isinstance(deck, dict):
            return {str(name): int(count) for name, count in deck.items()}
    return {}


def load_strategy(deck=None):
    """Return (strategy_text, path) for this deck, or (None, None).

    ARENA_STRATEGY_FILE always wins. Otherwise pick the strategy file in
    arena/strategies/ that mentions the most card names from the deck.
    With no deck to match, a single strategy file is used as-is.
    """
    explicit = os.environ.get("ARENA_STRATEGY_FILE")
    if explicit:
        path = Path(explicit)
        if path.is_file():
            return path.read_text(encoding="utf-8"), path
        return None, None

    if not STRATEGIES_DIR.is_dir():
        return None, None
    files = sorted(
        p for p in STRATEGIES_DIR.iterdir()
        if p.is_file() and p.suffix in (".md", ".txt")
    )
    if not files:
        return None, None

    texts = {p: p.read_text(encoding="utf-8") for p in files}
    if deck:
        names = [name.lower() for name in deck]
        scored = sorted(
            ((sum(1 for n in names if n in text.lower()), p) for p, text in texts.items()),
            key=lambda item: (-item[0], item[1].name),
        )
        score, best = scored[0]
        if score > 0:
            return texts[best], best
    if len(files) == 1:
        return texts[files[0]], files[0]
    return None, None


def build_system_prompt(deck=None, strategy=None):
    """Build the system prompt from the match deck and the designer's strategy."""
    if deck:
        deck_lines = "\n".join(
            f"{count} {name}" for name, count in sorted(deck.items(), key=lambda i: (-i[1], i[0]))
        )
        deck_section = f"The player's deck, read from the match log:\n{deck_lines}"
    else:
        deck_section = "The player's decklist has not been read from the match log yet."

    if strategy:
        strategy_section = (
            "The deck designer wrote this strategy in their own words. Follow it:\n\n"
            + strategy.strip()
        )
    else:
        strategy_section = "No strategy file matched this deck."

    return f"""You are an expert Magic: The Gathering Arena advisor.

{deck_section}

{strategy_section}

Rules for your advice:
- Follow the designer's strategy when it covers the situation.
- When the strategy does not cover the situation, or no strategy is available, do not guess silently: still pick the most defensible actionIndex, but say in your reasoning that the strategy does not cover this spot and ask the user what they want to do.
- Explain sequencing, mana, graveyard contents, opponent pressure, and the best alternative.
"""


class Alternative(BaseModel):
    action: int
    reasoning: str


class AdvisorResponse(BaseModel):
    recommended_action: int
    reasoning: str
    alternatives: list[Alternative]


class AdvisorError(RuntimeError):
    """Raised when the LLM advisor cannot produce a usable response."""


def _card_name(card):
    if isinstance(card, dict):
        return str(card.get("name") or card.get("cardName") or card.get("grpId") or "Unknown")
    return str(card)


def _zone_names(game_state, player_key, zone_key):
    player = game_state.get(player_key, {}) if isinstance(game_state, dict) else {}
    return [_card_name(card) for card in player.get(zone_key, [])]


def _action_name(action):
    if not isinstance(action, dict):
        return str(action)
    return str(action.get("name") or action.get("actionType") or action.get("actionIndex") or action)


def _action_index(action, fallback):
    if isinstance(action, dict):
        value = action.get("actionIndex", fallback)
    else:
        value = fallback
    try:
        return int(value)
    except (TypeError, ValueError):
        return fallback


def normalize_available_actions(available_actions):
    """Return action dicts with stable, unique actionIndex values.

    MTGA logs often omit actionIndex; mtga_state currently projects those as 0.
    In that case the advisor uses list position as the decision index.
    """
    indexes = []
    for i, action in enumerate(available_actions):
        indexes.append(_action_index(action, i))
    use_positions = len(indexes) != len(set(indexes))

    normalized = []
    for i, action in enumerate(available_actions):
        if isinstance(action, dict):
            item = dict(action)
        else:
            item = {"name": str(action), "actionType": ""}
        item["actionIndex"] = i if use_positions else _action_index(action, i)
        normalized.append(item)
    return normalized


def summarize_deck_composition(game_state):
    """Estimate library/graveyard composition from visible zones and the match deck."""
    graveyard = Counter(_zone_names(game_state, "you", "graveyard"))
    battlefield = Counter(_zone_names(game_state, "you", "battlefield"))
    exile = Counter(_zone_names(game_state, "you", "exile"))
    hand = Counter(_zone_names(game_state, "you", "hand"))
    summary = {
        "graveyard": dict(graveyard),
        "hand": dict(hand),
        "battlefield": dict(battlefield),
        "exile": dict(exile),
    }
    deck = deck_from_state(game_state)
    if deck:
        seen = graveyard + battlefield + exile + hand
        summary["library_estimate"] = {
            card: max(total - seen.get(card, 0), 0) for card, total in deck.items()
        }
    return summary


def build_user_prompt(game_state, available_actions, game_log, deck_composition):
    payload = {
        "game_state": game_state,
        "available_actions": available_actions,
        "recent_game_log": game_log[-30:] if isinstance(game_log, list) else game_log,
        "deck_composition": deck_composition,
    }
    return (
        "Choose the best available action for the current MTGA decision point. "
        "Use only actionIndex values from available_actions.\n\n"
        + json.dumps(payload, indent=2, sort_keys=True)
    )


def _request_kwargs(user_prompt, system_prompt=None):
    """Build the structured-output request shared by the sync and async paths."""
    if system_prompt is None:
        system_prompt = build_system_prompt(None, load_strategy()[0])
    return {
        "model": advisor_model(),
        "max_tokens": MAX_TOKENS,
        "system": [
            {
                "type": "text",
                "text": system_prompt,
                "cache_control": {"type": "ephemeral"},
            }
        ],
        "messages": [{"role": "user", "content": user_prompt}],
        "output_config": {"effort": advisor_effort()},
        "output_format": AdvisorResponse,
        "betas": [FALLBACK_BETA],
        "fallbacks": "default",
    }


def _extract_parsed(response):
    if response.stop_reason == "refusal":
        raise AdvisorError("the model declined to answer (stop_reason=refusal)")
    if response.stop_reason == "max_tokens":
        raise AdvisorError("the response was truncated (stop_reason=max_tokens)")
    parsed = response.parsed_output
    if parsed is None:
        raise AdvisorError("the model returned no structured output")
    return parsed


def _as_advisor_error(exc):
    if isinstance(exc, anthropic.AuthenticationError):
        return AdvisorError(f"Anthropic authentication failed: {exc}")
    if isinstance(exc, anthropic.RateLimitError):
        return AdvisorError(f"Anthropic rate limit reached: {exc}")
    if isinstance(exc, anthropic.APIStatusError):
        return AdvisorError(f"Anthropic API error {exc.status_code}: {exc}")
    if isinstance(exc, anthropic.APIConnectionError):
        return AdvisorError(f"Anthropic API connection failed: {exc}")
    return AdvisorError(f"Anthropic request failed: {exc}")


def _sync_client(timeout=REQUEST_TIMEOUT_SECONDS):
    return anthropic.Anthropic().with_options(timeout=timeout)


def _async_client(timeout=REQUEST_TIMEOUT_SECONDS):
    return anthropic.AsyncAnthropic().with_options(timeout=timeout)


def call_anthropic(user_prompt, timeout=REQUEST_TIMEOUT_SECONDS, system_prompt=None):
    """Call the Messages API and return a validated AdvisorResponse."""
    try:
        response = _sync_client(timeout).beta.messages.parse(
            **_request_kwargs(user_prompt, system_prompt)
        )
    except anthropic.AnthropicError as exc:
        raise _as_advisor_error(exc) from exc
    return _extract_parsed(response)


async def call_anthropic_async(user_prompt, timeout=REQUEST_TIMEOUT_SECONDS, system_prompt=None):
    """Async variant of call_anthropic for the live dashboard."""
    try:
        response = await _async_client(timeout).beta.messages.parse(
            **_request_kwargs(user_prompt, system_prompt)
        )
    except anthropic.AnthropicError as exc:
        raise _as_advisor_error(exc) from exc
    return _extract_parsed(response)


def _is_land_card(card):
    """Best-effort land check for a game-state card dict."""
    if isinstance(card, dict):
        card_types = card.get("cardTypes")
        if isinstance(card_types, list) and card_types:
            return "CardType_Land" in card_types
    name = _card_name(card)
    return any(land in name for land in LANDS)


def _mana_value(card):
    """Total mana value from a GRE manaCost list, or None when unknown."""
    if not isinstance(card, dict):
        return None
    cost = card.get("manaCost")
    if not isinstance(cost, list):
        return None
    total = 0
    for part in cost:
        if isinstance(part, dict):
            try:
                total += int(part.get("count", 0))
            except (TypeError, ValueError):
                pass
    return total


def _untapped_land_count(game_state):
    player = game_state.get("you", {}) if isinstance(game_state, dict) else {}
    count = 0
    for card in player.get("battlefield", []):
        if _is_land_card(card) and not (isinstance(card, dict) and card.get("tapped")):
            count += 1
    return count


def _action_cost(action, hand_cards):
    """Mana value an action would cost, or None when unknown.

    Prefer the action's own manaCost (it carries alternate costs such as
    impending); fall back to the matching hand card's printed cost.
    """
    if not isinstance(action, dict):
        return None
    own_cost = _mana_value(action)
    if own_cost is not None:
        return own_cost
    grp_id = action.get("grpId")
    name = action.get("name")
    for card in hand_cards:
        if not isinstance(card, dict):
            continue
        if grp_id and card.get("grpId") == grp_id:
            return _mana_value(card)
    for card in hand_cards:
        if isinstance(card, dict) and name and card.get("name") == name:
            return _mana_value(card)
    return None


def heuristic_recommendation(game_state, available_actions, reason_prefix="Offline heuristic"):
    """Small deterministic fallback so replay mode works without an API key."""
    player = game_state.get("you", {}) if isinstance(game_state, dict) else {}
    hand_cards = player.get("hand", [])
    hand = _zone_names(game_state, "you", "hand")
    graveyard = _zone_names(game_state, "you", "graveyard")
    opp_battlefield = _zone_names(game_state, "opponent", "battlefield")
    graveyard_targets = sum(1 for card in graveyard if any(target in card for target in BIG_TARGETS))
    hand_targets = sum(1 for card in hand if any(target in card for target in BIG_TARGETS))
    available_mana = _untapped_land_count(game_state)

    scored = []
    for fallback_index, action in enumerate(available_actions):
        idx = _action_index(action, fallback_index)
        name = _action_name(action)
        action_type = action.get("actionType", "") if isinstance(action, dict) else ""
        is_land_play = action_type in ("ActionType_Play", "ActionType_PlayLand") or (
            not action_type and any(card in name for card in LANDS)
        )
        is_cast = action_type.startswith("ActionType_Cast")
        score = 0.0
        reasons = []

        if is_cast:
            cost = _action_cost(action, hand_cards)
            if cost is not None and cost > available_mana:
                # Never recommend a spell we cannot pay for
                scored.append((-100.0, idx, name, f"cannot be paid for ({cost} mana vs {available_mana} untapped lands)"))
                continue

        if is_land_play:
            # Always make the land drop first
            score += 10.0
            reasons.append("develops mana (play a land before casting spells)")
        if any(card in name for card in ENABLERS):
            score += 5.0
            reasons.append("fills the graveyard for reanimation")
        if any(card in name for card in REANIMATORS):
            bonus = 2.0 + graveyard_targets * 1.7
            if "Bringer" in name and graveyard_targets >= 2:
                bonus += 2.5
            if "Kavaero" in name and graveyard_targets:
                bonus += 1.5
            score += bonus
            reasons.append(f"has {graveyard_targets} visible reanimation target(s)")
        if any(card in name for card in REMOVAL):
            score += 3.0 if opp_battlefield else 1.0
            if hand_targets:
                score += 1.0
                reasons.append("can discard a payoff if removal is not urgent")
            if opp_battlefield:
                reasons.append("answers opposing pressure")
        if "Beseech the Mirror" in name:
            score += 3.5
            reasons.append("can tutor for the missing engine piece")
        if "Pass" in name or action_type == "ActionType_Pass":
            score -= 1.0
            reasons.append("passes priority")
        if not reasons:
            reasons.append("legal action with lower deck synergy")

        scored.append((score, idx, name, "; ".join(reasons)))

    if not scored:
        return {
            "recommended_action": -1,
            "reasoning": f"{reason_prefix}: no available actions were supplied.",
            "alternatives": [],
        }

    scored.sort(key=lambda item: (item[0], -item[1]), reverse=True)
    best = scored[0]
    alternatives = [
        {"action": idx, "reasoning": f"{name}: {why}"}
        for _, idx, name, why in scored[1:4]
    ]
    return {
        "recommended_action": best[1],
        "reasoning": f"{reason_prefix}: {best[2]} is preferred because it {best[3]}.",
        "alternatives": alternatives,
    }


def normalize_recommendation(result, available_actions, game_state):
    valid = {_action_index(action, i) for i, action in enumerate(available_actions)}
    fallback = heuristic_recommendation(game_state, available_actions)
    if not isinstance(result, dict):
        return fallback
    try:
        recommended = int(result.get("recommended_action"))
    except (TypeError, ValueError):
        recommended = fallback["recommended_action"]
    if valid and recommended not in valid:
        recommended = fallback["recommended_action"]

    alternatives = []
    for alt in result.get("alternatives", []):
        if not isinstance(alt, dict):
            continue
        try:
            action = int(alt.get("action"))
        except (TypeError, ValueError):
            continue
        if valid and action not in valid:
            continue
        alternatives.append({"action": action, "reasoning": str(alt.get("reasoning", ""))})

    return {
        "recommended_action": recommended,
        "reasoning": str(result.get("reasoning") or fallback["reasoning"]),
        "alternatives": alternatives[:3] or fallback["alternatives"],
    }


def advise_move(game_state, available_actions, game_log=None, deck_composition=None, use_llm=True):
    """Return a move recommendation for a MTGA decision point.

    Args:
        game_state: dict from mtga_state.GameState.to_dict().
        available_actions: list of legal actions, each preferably with actionIndex.
        game_log: recent game events or play-by-play strings.
        deck_composition: cards remaining/visible by zone. Estimated if omitted.
        use_llm: when False, use the deterministic fallback only.
    """
    prepared = _prepare_llm_call(game_state, available_actions, game_log, deck_composition, use_llm)
    if isinstance(prepared, dict):
        return prepared
    actions, prompt, system_prompt = prepared
    try:
        parsed = call_anthropic(prompt, system_prompt=system_prompt)
    except AdvisorError as exc:
        return _llm_fallback(game_state, actions, exc)
    return normalize_recommendation(parsed.model_dump(), actions, game_state)


def _prepare_llm_call(game_state, available_actions, game_log, deck_composition, use_llm):
    """Return a heuristic result dict, or (actions, prompt, system_prompt) for an LLM call."""
    game_log = game_log or []
    actions = normalize_available_actions(available_actions)
    if not actions or not use_llm:
        return heuristic_recommendation(game_state, actions)
    deck_composition = deck_composition or summarize_deck_composition(game_state)
    deck = deck_from_state(game_state)
    strategy, _ = load_strategy(deck)
    system_prompt = build_system_prompt(deck, strategy)
    return actions, build_user_prompt(game_state, actions, game_log, deck_composition), system_prompt


def _llm_fallback(game_state, actions, exc):
    result = heuristic_recommendation(game_state, actions, reason_prefix="LLM unavailable; offline heuristic")
    result["reasoning"] += f" ({exc})"
    return result


def get_advice(game_state, actions, game_log=None, deck_info=None, use_llm=True):
    """Compatibility entry point for callers that ask for advisor advice."""
    return advise_move(game_state, actions, game_log, deck_info, use_llm=use_llm)


async def get_advice_async(game_state, actions, game_log=None, deck_info=None, use_llm=True):
    """Async advisor entry point for live dashboard use."""
    prepared = _prepare_llm_call(game_state, actions, game_log, deck_info, use_llm)
    if isinstance(prepared, dict):
        return prepared
    actions, prompt, system_prompt = prepared
    try:
        parsed = await call_anthropic_async(prompt, system_prompt=system_prompt)
    except AdvisorError as exc:
        return _llm_fallback(game_state, actions, exc)
    return normalize_recommendation(parsed.model_dump(), actions, game_state)


def _event_summary(event_type, data, state, card_db):
    if event_type == "match_state":
        gri = data.get("matchGameRoomStateChangedEvent", {}).get("gameRoomInfo", {})
        state_type = gri.get("stateType", "match state")
        if state_type == "MatchGameRoomStateType_Playing":
            names = ", ".join(state.player_names.get(s, f"Player#{s}") for s in sorted(state.player_names))
            return f"Match started: {names}" if names else "Match started"
        if state_type == "MatchGameRoomStateType_MatchCompleted":
            return "Match completed"
        return state_type
    if event_type == "client_msg":
        payload = data.get("payload", {})
        return payload.get("type", "client message")
    if event_type != "gre":
        return event_type

    summaries = []
    for msg in data.get("greToClientEvent", {}).get("greToClientMessages", []):
        msg_type = msg.get("type", "GRE message")
        if msg_type == "GREMessageType_ActionsAvailableReq":
            actions = msg.get("actionsAvailableReq", {}).get("actions", [])
            summaries.append(f"Actions available: {len(actions)}")
        elif msg_type == "GREMessageType_GameStateMessage":
            turn = state.turn_info.get("turnNumber")
            phase = state.turn_info.get("phase", "")
            summaries.append(f"Game state update turn={turn} phase={phase}")
        else:
            summaries.append(msg_type.replace("GREMessageType_", ""))
    return "; ".join(summaries) if summaries else "GRE event"


def _format_action(action, fallback_index=0):
    idx = _action_index(action, fallback_index)
    name = _action_name(action)
    action_type = action.get("actionType", "") if isinstance(action, dict) else ""
    suffix = f" ({action_type})" if action_type and action_type not in name else ""
    return f"[{idx}] {name}{suffix}"


def replay_log(log_path, use_llm=True, max_decisions=None):
    """Replay a Player.log and print advisor suggestions at each action prompt."""
    card_db = CardDatabase()
    state = GameState(card_db)
    recent_log = []
    decisions = 0
    deck_announced = False

    for event_type, data in parse_log(log_path):
        state.update(data)
        if not deck_announced and state.deck_grp_ids:
            deck = state.decklist()
            print(f"Deck read from match log ({sum(deck.values())} cards):")
            for name, count in sorted(deck.items(), key=lambda i: (-i[1], i[0])):
                print(f"  {count} {name}")
            _, strategy_path = load_strategy(deck)
            print(f"Strategy file: {strategy_path.name if strategy_path else 'none matched'}")
            deck_announced = True
        summary = _event_summary(event_type, data, state, card_db)
        if summary:
            recent_log.append(summary)
            recent_log = recent_log[-50:]

        actions = []
        if event_type == "gre":
            for msg in data.get("greToClientEvent", {}).get("greToClientMessages", []):
                if msg.get("type") == "GREMessageType_ActionsAvailableReq":
                    actions = normalize_available_actions(state.to_dict().get("actionsAvailable", []))
                    break
        if not actions:
            continue

        decisions += 1
        game_state = state.to_dict()
        recommendation = advise_move(
            game_state,
            actions,
            game_log=recent_log,
            deck_composition=summarize_deck_composition(game_state),
            use_llm=use_llm,
        )
        turn = game_state.get("turn", {})
        print(f"\nDecision {decisions}: turn {turn.get('number')} {turn.get('phase', '')} {turn.get('step', '')}".rstrip())
        print("Available actions:")
        for i, action in enumerate(actions):
            print(f"  {_format_action(action, i)}")
        print(f"Recommended action: {recommendation['recommended_action']}")
        print(f"Reasoning: {recommendation['reasoning']}")
        if recommendation.get("alternatives"):
            print("Alternatives:")
            for alt in recommendation["alternatives"]:
                print(f"  - {alt['action']}: {alt['reasoning']}")

        if max_decisions and decisions >= max_decisions:
            break

    if decisions == 0:
        print("No decision points with available actions found.")


def _load_json_arg(path_or_dash):
    if path_or_dash == "-":
        return json.load(sys.stdin)
    with open(path_or_dash, "r", encoding="utf-8") as f:
        return json.load(f)


def main(argv=None):
    parser = argparse.ArgumentParser(description="LLM move advisor for Dimir Reanimator in MTGA")
    parser.add_argument("--replay", metavar="PLAYER_LOG", help="walk a Player.log and advise at each decision point")
    parser.add_argument("--state", metavar="STATE_JSON", help="game state JSON file, or '-' for stdin")
    parser.add_argument("--actions", metavar="ACTIONS_JSON", help="available actions JSON file")
    parser.add_argument("--game-log", metavar="LOG_JSON", help="recent game log JSON file")
    parser.add_argument("--deck-composition", metavar="DECK_JSON", help="deck composition JSON file")
    parser.add_argument("--offline", action="store_true", help="skip Anthropic and use the deterministic fallback")
    parser.add_argument("--max-decisions", type=int, help="limit replay decisions, useful for smoke tests")
    args = parser.parse_args(argv)

    if args.replay:
        if not Path(args.replay).exists():
            parser.error(f"replay log not found: {args.replay}")
        replay_log(args.replay, use_llm=not args.offline, max_decisions=args.max_decisions)
        return 0

    if not args.state:
        parser.error("provide --replay or --state")
    game_state = _load_json_arg(args.state)
    actions = _load_json_arg(args.actions) if args.actions else game_state.get("actionsAvailable", [])
    game_log = _load_json_arg(args.game_log) if args.game_log else game_state.get("log", [])
    deck_composition = _load_json_arg(args.deck_composition) if args.deck_composition else None
    result = advise_move(game_state, actions, game_log, deck_composition, use_llm=not args.offline)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())