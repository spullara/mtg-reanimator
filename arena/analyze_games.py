#!/usr/bin/env python3
"""Extract the player's decisions from saved match captures and report the
strategy those games show.

Reads the capture files that arena/collect.py writes, replays each match's
raw GRE lines, and extracts the decisions the player made: mulligans,
lands (tapped or life paid), spells per turn, discard and mill choices,
Beseech the Mirror fetches and bargains, the combo turn and copy target,
interaction, and the result. Writes one decisions JSON per game, a
summary.json across games, and a plain-words report (report.md).

Offline by default. --llm sends the report and the summary — never the raw
logs, and no account names — to the Anthropic model the advisor uses.

    python arena/analyze_games.py [--captures DIR] [--out DIR] [--llm]
"""

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

# arena/__init__.py puts this folder on sys.path, so flat imports work both
# as a script and as `python -m arena.analyze_games`.
from collect import captures_dir
from log_tailer import LogEventParser
from parse_game import CardDatabase, GameState

# Card roles for this deck. Kept in sync with the role hints in advisor.py,
# defined here too so the offline path does not import the Anthropic SDK.
BIG_TARGETS = {
    "Bringer of the Last Gift",
    "Valgavoth, Terror Eater",
    "Terror of the Peaks",
    "Marang River Regent",
    "Deceit",
    "Ardyn, the Usurper",
}
COMBO_PIECE = "Superior Spider-Man"
TUTOR = "Beseech the Mirror"
INTERACTION = {"Bitter Triumph", "Grim Bauble", "Theorix Charm", "Coil and Catch"}

# The rules the Rust simulator's engine plays by (see src/ and
# arena/strategies/dimir-reanimator.md). The report counts the games where
# the player's decisions differ.
ENGINE_RULES = {
    "discard-target-first": "When a spell makes the player discard, discard a reanimation target first.",
    "combo-when-ready": "Cast Superior Spider-Man on the first turn the combo is ready.",
    "untapped-by-four": "Sequence lands so they enter untapped from turn four on.",
    "beseech-free-cast": "Bargain Beseech the Mirror only when the kill is at most two turns away.",
}


def events_from_raw_lines(raw_lines):
    """Re-parse a capture's raw log lines into (event_type, data) events."""
    parser = LogEventParser()
    return parser.feed("\n".join(raw_lines) + "\n")


class DecisionExtractor:
    """Walk one match's events and record the player's decisions."""

    def __init__(self, card_db=None):
        self.card_db = card_db or CardDatabase()

    def extract(self, events, metadata=None):
        metadata = metadata or {}
        state = GameState()
        card_db = self.card_db
        d = {
            "match_id": metadata.get("match_id"),
            "mulligans": 0,
            "kept_hand": [],
            "on_play": None,
            "lands": [],
            "spells": [],
            "discard_choices": [],
            "discarded": [],
            "milled": [],
            "beseech": [],
            "combo": None,
            "ready_turn": None,
            "interaction": [],
            "targets_chosen": [],
            "result": metadata.get("result", {}),
            "turns": metadata.get("turns"),
        }
        turn = 0
        seen_turns = set()
        last_bargain = None  # (turn, [sacrificed card names])
        pending_searches = {}  # SearchReq msgId -> source instance id
        beseech_fetch_ids = []  # (entry index, [instance ids]) resolved at the end
        self._my_turn_of = {}  # game turn number -> the player's own turn number
        self._my_turns = 0

        def our(seat):
            return state.our_seat is not None and seat == state.our_seat

        def name_of(instance_id):
            return state.get_card_name(instance_id, card_db)

        def our_zone_names(zone_type):
            names = []
            for zid, z in state.zones.items():
                if z.get("type") == zone_type and z.get("ownerSeatId") == state.our_seat:
                    for obj in state.objects.values():
                        if obj.get("zoneId") == zid:
                            names.append(card_db.get_name(obj.get("grpId", 0)))
            return names

        def our_battlefield_lands():
            count = 0
            for zid, z in state.zones.items():
                if z.get("type") != "ZoneType_Battlefield":
                    continue
                for obj in state.objects.values():
                    if obj.get("zoneId") != zid:
                        continue
                    controller = obj.get("controllerSeatId") or obj.get("ownerSeatId")
                    if controller == state.our_seat and "CardType_Land" in obj.get("cardTypes", []):
                        count += 1
            return count

        def spider_mana_value():
            for obj in state.objects.values():
                if card_db.get_name(obj.get("grpId", 0)) == COMBO_PIECE:
                    cost = obj.get("manaCost")
                    if isinstance(cost, list):
                        return sum(int(p.get("count", 0) or 0) for p in cost if isinstance(p, dict))
            return 4

        def check_ready():
            if d["ready_turn"] is not None:
                return
            hand = our_zone_names("ZoneType_Hand")
            graveyard = our_zone_names("ZoneType_Graveyard")
            # Lands that entered tapped this turn cannot pay for Spider-Man
            tapped_this_turn = sum(1 for land in d["lands"]
                                   if land["turn"] == turn and land["tapped"])
            if (COMBO_PIECE in hand
                    and any(c in BIG_TARGETS for c in graveyard)
                    and our_battlefield_lands() - tapped_this_turn >= spider_mana_value()):
                d["ready_turn"] = turn

        pending_select_n = {}

        for event_type, data in events:
            if event_type == "gre":
                for msg in data.get("greToClientEvent", {}).get("greToClientMessages", []):
                    msg_type = msg.get("type", "")
                    if msg_type == "GREMessageType_ConnectResp":
                        seats = msg.get("systemSeatIds", [])
                        if seats:
                            state.our_seat = seats[0]
                    elif msg_type == "GREMessageType_SelectNReq":
                        pending_select_n[msg.get("msgId")] = msg
                    elif msg_type == "GREMessageType_SearchReq":
                        params = msg.get("prompt", {}).get("parameters", [])
                        source_id = next((pp.get("numberValue") for pp in params
                                          if pp.get("parameterName") == "CardId"), None)
                        pending_searches[msg.get("msgId")] = source_id
                    elif msg_type in ("GREMessageType_GameStateMessage",
                                      "GREMessageType_QueuedGameStateMessage"):
                        gsm = msg.get("gameStateMessage", {})
                        turn = self._process_gsm(gsm, state, d, turn, seen_turns, our, name_of)
                        if state.turn_info.get("activePlayer") == state.our_seat:
                            check_ready()

            elif event_type == "client_msg":
                payload = data.get("payload", {})
                msg_type = payload.get("type", "")

                if msg_type == "ClientMessageType_MulliganResp":
                    decision = payload.get("mulliganResp", {}).get("decision", "")
                    if decision == "MulliganOption_AcceptHand":
                        d["kept_hand"] = our_zone_names("ZoneType_Hand")
                    else:
                        d["mulligans"] += 1

                elif msg_type == "ClientMessageType_CastingTimeOptionsResp":
                    choice = payload.get("castingTimeOptionsResp", {}).get("castingTimeOptionResp", {})
                    if "Bargain" in str(choice.get("castingTimeOptionType", "")):
                        last_bargain = (turn, [])

                elif msg_type == "ClientMessageType_EffectCostResp":
                    ids = payload.get("effectCostResp", {}).get("costSelection", {}).get("ids", [])
                    if last_bargain and last_bargain[0] == turn:
                        last_bargain = (turn, [name_of(i) for i in ids])

                elif msg_type == "ClientMessageType_SelectTargetsResp":
                    for t in payload.get("selectTargetsResp", {}).get("target", {}).get("targets", []):
                        tid = t.get("targetInstanceId")
                        if tid:
                            target = state.get_player_name(tid) if tid in (1, 2) else name_of(tid)
                            d["targets_chosen"].append({"turn": turn, "target": target})

                elif msg_type == "ClientMessageType_SearchResp":
                    source_id = pending_searches.pop(payload.get("respId"), None)
                    found = payload.get("searchResp", {}).get("itemsFound", [])
                    bargained = bool(last_bargain and last_bargain[0] == turn)
                    if found and (bargained or (source_id and name_of(source_id) == TUTOR)):
                        d["beseech"].append({
                            "turn": turn,
                            "fetched": None,
                            "bargained": bargained,
                            "sacrificed": last_bargain[1] if bargained else [],
                        })
                        beseech_fetch_ids.append((len(d["beseech"]) - 1, found))

                elif msg_type == "ClientMessageType_SelectNResp":
                    req = pending_select_n.pop(payload.get("respId"), {})
                    ids = payload.get("selectNResp", {}).get("ids", [])
                    if not req or not ids:
                        continue
                    prompt_text = card_db.get_prompt_text(req.get("prompt", {}).get("promptId"))
                    source_id = req.get("selectNReq", {}).get("sourceId")
                    source = name_of(source_id) if source_id else "effect"
                    chosen = [name_of(i) for i in ids]
                    if "Discard" in prompt_text:
                        d["discard_choices"].append({
                            "turn": turn, "source": source, "cards": chosen,
                        })
                    elif prompt_text == "Choose a nonland card.":
                        # Hand attack (for example Deceit evoked with {B}{B}):
                        # the player picks a card from the opponent's hand
                        d["interaction"].append({
                            "turn": turn, "card": source, "effect": "hand attack",
                            "took": chosen[0],
                        })
                    elif prompt_text == "Select object to copy.":
                        d["combo"] = {"turn": turn, "copy_target": chosen[0]}

        for idx, ids in beseech_fetch_ids:
            d["beseech"][idx]["fetched"] = name_of(ids[0])

        # Report the player's own turn numbers ("your turn 5"), not the game's
        # turn counter, which counts both players' turns.
        def mine(t):
            return self._my_turn_of.get(t, t) if t else t
        for key in ("lands", "spells", "discard_choices", "discarded", "milled",
                    "beseech", "interaction", "targets_chosen"):
            for entry in d[key]:
                entry["turn"] = mine(entry.get("turn"))
        d["ready_turn"] = mine(d["ready_turn"])
        if d["combo"]:
            d["combo"]["turn"] = mine(d["combo"]["turn"])
        d["game_turns"] = d.get("turns")
        d["turns"] = self._my_turns or d.get("turns")

        if d["combo"] and d["ready_turn"] is not None:
            d["combo"]["ready_turn"] = d["ready_turn"]
            d["combo"]["delay"] = max(d["combo"]["turn"] - d["ready_turn"], 0)
        return d

    def _process_gsm(self, gsm, state, d, turn, seen_turns, our, name_of):
        card_db = self.card_db
        # Life delta for the player, read before the state absorbs the update
        life_delta = 0
        damage_to_us = False
        for p in gsm.get("players", []):
            seat = p.get("systemSeatNumber") or p.get("seatId")
            if our(seat) and p.get("lifeTotal") is not None:
                old = state.players.get(seat, {}).get("lifeTotal")
                if old is not None:
                    life_delta = p["lifeTotal"] - old
        for ann in gsm.get("annotations", []):
            ann_types = ann.get("type", [])
            if "AnnotationType_DamageDealt" in ann_types and state.our_seat in ann.get("affectedIds", []):
                damage_to_us = True
            if "AnnotationType_ObjectIdChanged" in ann_types:
                details = {dd["key"]: dd for dd in ann.get("details", [])}
                orig_id = details.get("orig_id", {}).get("valueInt32", [None])[0]
                new_id = details.get("new_id", {}).get("valueInt32", [None])[0]
                if orig_id and new_id:
                    state.id_changes[new_id] = orig_id

        state.update_from_game_state_msg(gsm)

        ti = gsm.get("turnInfo", {})
        if ti.get("turnNumber") and ti["turnNumber"] not in seen_turns:
            turn = ti["turnNumber"]
            seen_turns.add(turn)
            if ti.get("activePlayer") == state.our_seat:
                self._my_turns += 1
            self._my_turn_of[turn] = self._my_turns
            if turn == 1 and ti.get("activePlayer"):
                d["on_play"] = ti["activePlayer"] == state.our_seat
        elif ti.get("turnNumber"):
            turn = ti["turnNumber"]

        for ann in gsm.get("annotations", []):
            if "AnnotationType_ZoneTransfer" not in ann.get("type", []):
                continue
            details = {dd["key"]: dd for dd in ann.get("details", [])}
            category = details.get("category", {}).get("valueString", [""])[0]
            for inst_id in ann.get("affectedIds", []):
                owner = state.get_object_owner(inst_id)
                if not our(owner):
                    continue
                card = name_of(inst_id)
                if category == "PlayLand":
                    obj = state._resolve_object(inst_id) or {}
                    d["lands"].append({
                        "turn": turn,
                        "card": card,
                        "tapped": bool(obj.get("isTapped")),
                        "life_paid": -life_delta if (life_delta < 0 and not damage_to_us) else 0,
                    })
                elif category == "CastSpell":
                    d["spells"].append({"turn": turn, "card": card})
                    if card in INTERACTION:
                        d["interaction"].append({"turn": turn, "card": card})
                elif category == "Discard":
                    d["discarded"].append({"turn": turn, "card": card})
                elif category == "Mill":
                    d["milled"].append({"turn": turn, "card": card})
        return turn


def summarize(games):
    """Aggregate per-game decisions into one summary dict."""
    summary = {
        "games": len(games),
        "wins": sum(1 for g in games if g.get("result", {}).get("won")),
        "mulligans": sum(g["mulligans"] for g in games),
        "average_turns": _avg([g["turns"] for g in games if g.get("turns")]),
        "combo": [],
        "discard_first": Counter(),
        "discard_choices": 0,
        "target_first": 0,
        "bringer_first": 0,
        "lands_tapped_by_turn": {},
        "life_paid_total": 0,
        "beseech_fetches": Counter(),
        "beseech_bargains": 0,
        "beseech_casts": 0,
        "interaction_casts": Counter(),
        "rule_differences": [],
    }
    tapped = {}
    for g in games:
        gid = g.get("match_id", "?")
        if g.get("combo"):
            summary["combo"].append({
                "match_id": gid,
                "turn": g["combo"]["turn"],
                "copy_target": g["combo"]["copy_target"],
                "ready_turn": g["combo"].get("ready_turn"),
                "delay": g["combo"].get("delay"),
            })
        for choice in g["discard_choices"]:
            summary["discard_choices"] += 1
            first = choice["cards"][0]
            summary["discard_first"][first] += 1
            if first in BIG_TARGETS:
                summary["target_first"] += 1
            if first == "Bringer of the Last Gift":
                summary["bringer_first"] += 1
        for land in g["lands"]:
            tapped.setdefault(land["turn"], [0, 0])
            tapped[land["turn"]][0 if land["tapped"] else 1] += 1
            summary["life_paid_total"] += land.get("life_paid", 0)
        for b in g["beseech"]:
            summary["beseech_casts"] += 1
            summary["beseech_fetches"][b["fetched"]] += 1
            if b["bargained"]:
                summary["beseech_bargains"] += 1
        for i in g["interaction"]:
            label = i["card"] + (f" ({i['effect']})" if i.get("effect") else "")
            summary["interaction_casts"][label] += 1
    summary["lands_tapped_by_turn"] = {
        str(t): {"tapped": v[0], "untapped": v[1]} for t, v in sorted(tapped.items())
    }
    summary["discard_first"] = dict(summary["discard_first"])
    summary["beseech_fetches"] = dict(summary["beseech_fetches"])
    summary["interaction_casts"] = dict(summary["interaction_casts"])
    summary["rule_differences"] = rule_differences(games, summary)
    return summary


def _avg(values):
    return round(sum(values) / len(values), 1) if values else None


def rule_differences(games, summary):
    """Compare the observed decisions with the simulator's engine rules."""
    diffs = []
    if summary["discard_choices"]:
        misses = summary["discard_choices"] - summary["target_first"]
        if misses:
            diffs.append(
                f"discard-target-first: a reanimation target was NOT discarded first "
                f"in {misses} of {summary['discard_choices']} forced discards."
            )
    for c in summary["combo"]:
        if c.get("delay"):
            diffs.append(
                f"combo-when-ready: the combo was ready on turn {c['ready_turn']} "
                f"but Superior Spider-Man was cast on turn {c['turn']} "
                f"({c['delay']} turn(s) later)."
            )
    late_tapped = sum(
        1 for g in games for land in g["lands"] if land["turn"] >= 4 and land["tapped"]
    )
    if late_tapped:
        diffs.append(
            f"untapped-by-four: {late_tapped} land(s) entered tapped on turn four or later."
        )
    if summary["beseech_bargains"]:
        diffs.append(
            f"beseech-free-cast: Beseech was bargained in {summary['beseech_bargains']} "
            f"of {summary['beseech_casts']} casts; the engine only free-casts when the "
            f"kill is at most two turns away (lethal distance is not visible in the log)."
        )
    return diffs


def write_report(games, summary):
    """Build the plain-words strategy report. Never includes account names."""
    lines = []
    out = lines.append
    out(f"# Strategy report — {summary['games']} game(s)")
    out("")
    out("## Results")
    out(f"- The player won {summary['wins']} of {summary['games']} game(s).")
    if summary["average_turns"]:
        out(f"- Average game length: {summary['average_turns']} of the player's turns.")
    out("")
    out("## Mulligans")
    keeps = summary["games"] - sum(1 for g in games if g["mulligans"])
    out(f"- Kept the opening hand in {keeps} of {summary['games']} game(s) "
        f"({summary['mulligans']} mulligan(s) total).")
    out("")
    out("## Combo")
    if summary["combo"]:
        for c in summary["combo"]:
            ready = (f" The pieces were ready on turn {c['ready_turn']}"
                     + (f" — a delay of {c['delay']} turn(s)." if c.get("delay")
                        else " (no delay).")) if c.get("ready_turn") else ""
            out(f"- Cast Superior Spider-Man on turn {c['turn']}, copying "
                f"{c['copy_target']}.{ready}")
    else:
        out("- The combo was not cast in these games.")
    out("")
    out("## Forced discards")
    if summary["discard_choices"]:
        out(f"- Discarded a reanimation target first in {summary['target_first']} "
            f"of {summary['discard_choices']} choice(s).")
        out(f"- Bringer of the Last Gift went first in {summary['bringer_first']} "
            f"of {summary['discard_choices']}.")
        for card, count in sorted(summary["discard_first"].items(), key=lambda i: -i[1]):
            out(f"- First discard was {card} in {count} choice(s).")
    else:
        out("- No forced discard choices were recorded.")
    out("")
    out("## Lands")
    for t, counts in summary["lands_tapped_by_turn"].items():
        out(f"- Turn {t}: {counts['untapped']} untapped, {counts['tapped']} tapped.")
    if summary["life_paid_total"]:
        out(f"- Paid {summary['life_paid_total']} life to lands in total.")
    out("")
    out("## Beseech the Mirror")
    if summary["beseech_casts"]:
        for card, count in sorted(summary["beseech_fetches"].items(), key=lambda i: -i[1]):
            out(f"- Fetched {card} in {count} cast(s).")
        out(f"- Bargained in {summary['beseech_bargains']} of {summary['beseech_casts']} cast(s).")
    else:
        out("- Beseech the Mirror was not cast in these games.")
    out("")
    out("## Interaction")
    if summary["interaction_casts"]:
        for card, count in sorted(summary["interaction_casts"].items(), key=lambda i: -i[1]):
            out(f"- Cast {card} {count} time(s).")
    else:
        out("- No interaction was used.")
    out("")
    out("## Decisions that differ from the simulator's engine rules")
    if summary["rule_differences"]:
        for diff in summary["rule_differences"]:
            out(f"- {diff}")
    else:
        out("- None found: the recorded decisions match the engine rules.")
    out("")
    out("Engine rules checked:")
    for key, text in ENGINE_RULES.items():
        out(f"- {key}: {text}")
    return "\n".join(lines)


LLM_PROMPT = (
    "You are a Magic: The Gathering deck coach. Below is a report extracted "
    "from a player's saved MTG Arena games with a Dimir reanimator deck, "
    "plus the summary data. Discuss the strategy the games show: what the "
    "player does well, where their decisions differ from the simulator's "
    "engine rules, and which habit is most worth changing. Be concrete and "
    "reference the counts.\n\n"
)


def llm_discussion(report_text, summary):
    """Send the scrubbed report through the advisor's Anthropic client."""
    import advisor

    client = advisor._sync_client(timeout=120.0)
    response = client.messages.create(
        model=advisor.advisor_model(),
        max_tokens=2000,
        messages=[{
            "role": "user",
            "content": LLM_PROMPT + report_text + "\n\nSummary data:\n"
            + json.dumps(summary, indent=2, sort_keys=True),
        }],
    )
    return "".join(
        block.text for block in response.content if getattr(block, "type", "") == "text"
    )


def load_captures(folder):
    docs = []
    for path in sorted(Path(folder).glob("match_*.json")):
        try:
            docs.append(json.loads(path.read_text(encoding="utf-8")))
        except (OSError, json.JSONDecodeError) as exc:
            print(f"Skipping {path}: {exc}", file=sys.stderr)
    return docs


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Analyze saved MTGA matches and report the player's strategy"
    )
    parser.add_argument("--captures", "--dir", dest="captures", metavar="DIR",
                        help="folder with match_*.json captures (default: arena/captures/)")
    parser.add_argument("--out", metavar="DIR",
                        help="output folder for decisions/summary/report (default: same as captures)")
    parser.add_argument("--card-db", metavar="DB",
                        help="path to Raw_CardDatabase_*.mtga (default: auto-detect)")
    parser.add_argument("--llm", action="store_true",
                        help="also send the report (no logs, no names) to the advisor model")
    args = parser.parse_args(argv)

    src = captures_dir(args.captures)
    out_dir = Path(args.out) if args.out else Path(src)
    out_dir.mkdir(parents=True, exist_ok=True)

    docs = load_captures(src)
    if not docs:
        print(f"No captures found in {src}. Run `python -m arena.collect --once` first.",
              file=sys.stderr)
        return 1

    card_db = CardDatabase(args.card_db)
    extractor = DecisionExtractor(card_db)
    games = []
    for doc in docs:
        events = events_from_raw_lines(doc.get("raw_lines", []))
        decisions = extractor.extract(events, metadata=doc)
        games.append(decisions)
        path = out_dir / f"decisions_{doc['match_id']}.json"
        path.write_text(json.dumps(decisions, indent=1), encoding="utf-8")
        print(f"Wrote {path}")

    summary = summarize(games)
    summary_path = out_dir / "summary.json"
    summary_path.write_text(json.dumps(summary, indent=1, sort_keys=True), encoding="utf-8")
    print(f"Wrote {summary_path}")

    report = write_report(games, summary)
    report_path = out_dir / "report.md"
    report_path.write_text(report + "\n", encoding="utf-8")
    print(f"Wrote {report_path}\n")
    print(report)

    if args.llm:
        print("\n# Model discussion\n")
        print(llm_discussion(report, summary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
