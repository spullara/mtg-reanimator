#!/usr/bin/env python3
"""Parse MTGA Player.log to produce a human-readable play-by-play of matches."""

import argparse
import json
import os
import re
import sqlite3
import sys
from pathlib import Path

from paths import find_card_db, find_player_log

PHASE_NAMES = {
    "Phase_Beginning": "Beginning",
    "Phase_Main1": "Main 1",
    "Phase_Combat": "Combat",
    "Phase_Main2": "Main 2",
    "Phase_Ending": "End",
}
STEP_NAMES = {
    "Step_Upkeep": "Upkeep",
    "Step_Draw": "Draw",
    "Step_BeginCombat": "Begin Combat",
    "Step_DeclareAttack": "Declare Attackers",
    "Step_DeclareBlock": "Declare Blockers",
    "Step_CombatDamage": "Combat Damage",
    "Step_FirstStrikeDamage": "First Strike Damage",
    "Step_EndCombat": "End Combat",
    "Step_EndStep": "End Step",
}
ZONE_NAMES = {
    "ZoneType_Hand": "Hand",
    "ZoneType_Library": "Library",
    "ZoneType_Battlefield": "Battlefield",
    "ZoneType_Graveyard": "Graveyard",
    "ZoneType_Stack": "Stack",
    "ZoneType_Exile": "Exile",
    "ZoneType_Limbo": "Limbo",
    "ZoneType_Command": "Command",
    "ZoneType_Revealed": "Revealed",
    "ZoneType_Sideboard": "Sideboard",
    "ZoneType_Pending": "Pending",
    "ZoneType_Suppressed": "Suppressed",
}
CATEGORY_VERBS = {
    "PlayLand": "Play",
    "CastSpell": "Cast",
    "Resolve": "Resolve",
    "Draw": "Draw",
    "Mill": "Mill",
    "Discard": "Discard",
    "Destroy": "Destroy",
    "Exile": "Exile",
    "ReturnToHand": "Return to hand",
    "PutOnBattlefield": "Put onto battlefield",
    "Sacrifice": "Sacrifice",
    "Counter": "Counter",
}
COLOR_SYMBOLS = {
    "ManaColor_White": "W",
    "ManaColor_Blue": "U",
    "ManaColor_Black": "B",
    "ManaColor_Red": "R",
    "ManaColor_Green": "G",
    "ManaColor_Colorless": "C",
    "ManaColor_Generic": "1",
    "ManaColor_AnyColor": "any",
}


def clean_enum(value, prefix=""):
    """Make GRE enum strings readable."""
    if not value:
        return ""
    if prefix and value.startswith(prefix):
        value = value[len(prefix):]
    return value.replace("_", " ")


def format_cost_string(cost):
    """Format MTGA's o1oBo(U/B) style cost strings."""
    if not cost:
        return ""
    cost = re.sub(r"o\(([^)]+)\)", r"{\1}", cost)
    cost = re.sub(r"o([0-9A-Z])", r"{\1}", cost)
    return cost


def format_mana_cost(mana_cost):
    """Format GRE manaCost arrays as compact mana symbols."""
    if not mana_cost:
        return ""
    symbols = []
    for cost in mana_cost:
        colors = cost.get("color", []) or []
        count = cost.get("count", 1) or 1
        if not colors:
            continue
        if colors == ["ManaColor_Generic"]:
            symbols.append(f"{{{count}}}")
            continue
        symbol = "/".join(COLOR_SYMBOLS.get(c, clean_enum(c, "ManaColor_")) for c in colors)
        symbols.extend([f"{{{symbol}}}"] * count)
    return "".join(symbols)


def response_word(response):
    if response in ("OptionResponse_Cancel_No", "OptionResponse_No"):
        return "Decline"
    if response in ("OptionResponse_Accept_Yes", "OptionResponse_Yes"):
        return "Accept"
    return clean_enum(response, "OptionResponse_") or "Respond"


# Known GRE prompt texts, used when the card database is not installed.
PROMPT_TEXT_FALLBACK = {
    78: "Select object to copy.",
    1032: "Choose a nonland card.",
    1034: "Discard two cards.",
}


class CardDatabase:
    """Look up card names and prompt texts from the MTGA CardDatabase SQLite file."""

    def __init__(self, db_path=None):
        self._cache = {}
        self._prompt_cache = {}
        found = find_card_db(db_path)
        self._db_path = str(found) if found else None

    def get_prompt_text(self, prompt_id):
        """Return the English text of a GRE promptId, or '' if unknown."""
        if not prompt_id:
            return ""
        if prompt_id in self._prompt_cache:
            return self._prompt_cache[prompt_id]
        text = ""
        if self._db_path:
            try:
                conn = sqlite3.connect(self._db_path)
                cur = conn.cursor()
                cur.execute(
                    "SELECT l.Loc FROM Prompts p JOIN Localizations_enUS l "
                    "ON p.LocId = l.LocId WHERE p.Id = ?",
                    (prompt_id,),
                )
                row = cur.fetchone()
                conn.close()
                if row:
                    text = row[0]
            except Exception:
                pass
        if not text:
            text = PROMPT_TEXT_FALLBACK.get(prompt_id, "")
        self._prompt_cache[prompt_id] = text
        return text

    def get_name(self, grp_id):
        if grp_id in self._cache:
            return self._cache[grp_id]
        if grp_id == 0:
            return "Unknown"
        if self._db_path:
            try:
                conn = sqlite3.connect(self._db_path)
                cur = conn.cursor()
                cur.execute(
                    "SELECT l.Loc FROM Cards c JOIN Localizations_enUS l "
                    "ON c.TitleId = l.LocId WHERE c.GrpId = ?",
                    (grp_id,),
                )
                row = cur.fetchone()
                conn.close()
                if row:
                    self._cache[grp_id] = row[0]
                    return row[0]
            except Exception:
                pass
        name = f"#{grp_id}"
        self._cache[grp_id] = name
        return name


class GameState:
    """Track evolving game state across diff messages."""

    def __init__(self):
        self.zones = {}       # zoneId -> zone dict
        self.objects = {}     # instanceId -> object dict
        self.players = {}     # seatId -> player dict
        self.turn_info = {}
        self.match_id = None
        self.player_names = {}  # seatId -> name
        self.our_seat = None
        self.event_id = None
        self.mulligan_count = 0
        self.pending_actions = []  # most recent ActionsAvailableReq actions list
        self.pending_select_targets = {}  # msgId -> SelectTargetsReq data
        self.pending_select_n = {}  # msgId -> SelectNReq data
        self.pending_casting_time_options = {}  # msgId -> CastingTimeOptionsReq data
        self.pending_pay_costs = {}  # msgId -> PayCostsReq data
        self.pending_searches = {}  # msgId -> SearchReq data
        self.pending_orders = {}  # msgId -> OrderReq data
        self.pending_optional_actions = {}  # msgId -> OptionalActionMessage data
        self.pending_attackers = {}  # msgId -> DeclareAttackersReq data
        self.opening_hand_shown = False
        self.die_roll_shown = False
        self.damage_annotation_ids = set()  # track already-shown damage annotations
        self.deleted_objects = {}  # instanceId -> object dict (preserved after deletion)
        self.pending_blockers = []  # buffered blocking assignments (final state only)
        self.token_creators = {}  # token instanceId -> creator instanceId
        self.id_changes = {}  # new instanceId -> orig instanceId

    def update_from_game_state_msg(self, gsm):
        if gsm.get("zones"):
            for z in gsm["zones"]:
                self.zones[z["zoneId"]] = z
        if gsm.get("gameObjects"):
            for obj in gsm["gameObjects"]:
                self.objects[obj["instanceId"]] = obj
        if gsm.get("players"):
            for p in gsm["players"]:
                seat = p.get("systemSeatNumber") or p.get("seatId")
                if seat:
                    self.players[seat] = p
        if gsm.get("turnInfo"):
            self.turn_info = gsm["turnInfo"]
        if gsm.get("gameInfo"):
            gi = gsm["gameInfo"]
            if "matchID" in gi:
                self.match_id = gi["matchID"]
        # Handle deleted instance IDs — preserve in deleted_objects for name resolution
        for did in gsm.get("diffDeletedInstanceIds", []):
            obj = self.objects.pop(did, None)
            if obj:
                self.deleted_objects[did] = obj

    def get_zone_name(self, zone_id):
        z = self.zones.get(zone_id)
        if z:
            return ZONE_NAMES.get(z["type"], z["type"])
        return f"Zone#{zone_id}"

    def _resolve_object(self, instance_id):
        """Find the object for an instanceId, following id_changes if needed."""
        obj = self.objects.get(instance_id) or self.deleted_objects.get(instance_id)
        if obj:
            return obj
        # Follow ID change chain: new_id -> orig_id
        orig_id = self.id_changes.get(instance_id)
        if orig_id:
            return self.objects.get(orig_id) or self.deleted_objects.get(orig_id)
        return None

    def get_card_name(self, instance_id, card_db):
        obj = self._resolve_object(instance_id)
        if obj:
            grp_id = obj.get("grpId", 0)
            name = card_db.get_name(grp_id)
            # If DB didn't resolve a real name, check if it's a token with a creator
            if name in ("Unknown", f"#{grp_id}") and instance_id in self.token_creators:
                creator_id = self.token_creators[instance_id]
                creator_obj = self._resolve_object(creator_id)
                if creator_obj:
                    creator_name = card_db.get_name(creator_obj.get("grpId", 0))
                    if creator_name and creator_name not in ("Unknown",):
                        return f"token (from {creator_name})"
                return "token"
            return name
        # Not in objects at all — check if it's a known token
        if instance_id in self.token_creators:
            creator_id = self.token_creators[instance_id]
            creator_obj = self._resolve_object(creator_id)
            if creator_obj:
                creator_name = card_db.get_name(creator_obj.get("grpId", 0))
                if creator_name and creator_name not in ("Unknown",):
                    return f"token (from {creator_name})"
            return "token"
        return "Unknown permanent"

    def get_object_owner(self, instance_id):
        """Get ownerSeatId for an instance, following id changes and checking deleted objects."""
        obj = self._resolve_object(instance_id)
        if obj:
            return obj.get("ownerSeatId")
        # For tokens not in objects, try to inherit owner from creator
        creator_id = self.token_creators.get(instance_id)
        if creator_id:
            creator_obj = self._resolve_object(creator_id)
            if creator_obj:
                return creator_obj.get("ownerSeatId")
        return None

    def get_player_name(self, seat_id):
        return self.player_names.get(seat_id, f"Player#{seat_id}")

    def get_hand_zone_id(self, seat_id):
        """Find the hand zone for a given seat."""
        for zid, z in self.zones.items():
            if z.get("type") == "ZoneType_Hand" and z.get("ownerSeatId") == seat_id:
                return zid
        return None

    def get_objects_in_zone(self, zone_id):
        """Get all tracked objects in a given zone."""
        return [obj for obj in self.objects.values() if obj.get("zoneId") == zone_id]


def parse_log(log_path):
    """Parse the Player.log file and yield (event_type, data) tuples.

    event_type: 'gre', 'match_state', 'client_msg'
    """
    with open(log_path, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    i = 0
    while i < len(lines):
        line = lines[i].rstrip("\n")

        # Server→Client GRE messages and match state (single-line JSON)
        if line.startswith("{ ") or line.startswith('{"'):
            try:
                data = json.loads(line)
                if "greToClientEvent" in data:
                    yield ("gre", data)
                elif "matchGameRoomStateChangedEvent" in data:
                    yield ("match_state", data)
            except json.JSONDecodeError:
                pass
            i += 1
            continue

        # Client→Server messages (multi-line JSON after header)
        if "ClientToGremessage" in line or "ClientToGreuimessage" in line:
            # Accumulate multi-line JSON
            json_lines = []
            i += 1
            brace_depth = 0
            while i < len(lines):
                jl = lines[i].rstrip("\n")
                json_lines.append(jl)
                brace_depth += jl.count("{") - jl.count("}")
                if brace_depth <= 0 and json_lines:
                    break
                i += 1
            if json_lines:
                try:
                    data = json.loads("\n".join(json_lines))
                    yield ("client_msg", data)
                except json.JSONDecodeError:
                    pass
            i += 1
            continue

        i += 1


def render_events(events, card_db=None):
    """Render parsed log events into play-by-play lines and return them."""
    card_db = card_db or CardDatabase()
    state = GameState()
    prev_turn = None
    prev_phase = None
    prev_step = None
    player_turn_counts = {}  # seatId -> per-player turn counter
    in_match = False
    output_lines = []
    # Persistent pending headers — survive across GSMs until flushed by content
    pending_turn_header = None
    pending_phase_header = None

    def out(s=""):
        output_lines.append(s)

    def flush_headers():
        nonlocal pending_turn_header, pending_phase_header
        if pending_turn_header:
            out(f"\n--- {pending_turn_header} ---")
            for seat in sorted(state.players):
                p = state.players[seat]
                life = p.get("lifeTotal", "?")
                out(f"  {state.get_player_name(seat)}: {life} life")
            pending_turn_header = None
        if pending_phase_header:
            out(f"  [{pending_phase_header}]")
            pending_phase_header = None

    def msg_player_name(msg):
        seats = msg.get("systemSeatIds", [])
        seat = seats[0] if seats else state.our_seat
        return state.get_player_name(seat) if seat else "Player"

    def describe_object(instance_id):
        if instance_id in (1, 2):
            return state.get_player_name(instance_id)
        return state.get_card_name(instance_id, card_db) if instance_id else "Unknown"

    def card_list(ids):
        return [describe_object(i) for i in ids]

    def first_prompt_card(prompt):
        for param in prompt.get("parameters", []):
            if param.get("parameterName") == "CardId" and param.get("numberValue"):
                return describe_object(param["numberValue"])
        return ""

    def prompt_player(prompt):
        for param in prompt.get("parameters", []):
            ref = param.get("reference", {})
            if ref.get("type") == "ReferenceType_PlayerSeatId":
                return state.get_player_name(ref.get("id"))
        return ""

    def pay_cost_label(req):
        prompt = req.get("prompt", {})
        for param in prompt.get("parameters", []):
            if param.get("parameterName") == "Cost" and param.get("stringValue"):
                return format_cost_string(param["stringValue"])
        return format_mana_cost(req.get("payCostsReq", {}).get("manaCost", []))

    def pay_cost_source(req):
        pc = req.get("payCostsReq", {})
        for cost in pc.get("manaCost", []) or []:
            object_id = cost.get("objectId")
            if object_id:
                return describe_object(object_id)
        prompt_card = first_prompt_card(req.get("prompt", {}))
        return prompt_card or "cost"

    def format_auto_tap(req):
        solutions = req.get("payCostsReq", {}).get("autoTapActionsReq", {}).get("autoTapSolutions", [])
        if not solutions:
            return ""
        actions = solutions[0].get("autoTapActions", [])
        pieces = []
        for action in actions:
            mana = action.get("manaPaymentOption", {}).get("mana", [])
            source_id = action.get("instanceId")
            if not source_id and mana:
                source_id = mana[0].get("srcInstanceId")
            source = describe_object(source_id) if source_id else "mana"
            mana_text = "".join(
                f"{{{COLOR_SYMBOLS.get(m.get('color'), clean_enum(m.get('color'), 'ManaColor_'))}}}"
                for m in mana
                for _ in range(m.get("count", 1) or 1)
            )
            pieces.append(f"{source} for {mana_text}" if mana_text else source)
        return ", ".join(pieces)

    def casting_option_label(option):
        option_type = clean_enum(option.get("castingTimeOptionType"), "CastingTimeOptionType_")
        if option_type == "Done":
            return "normal timing"
        target = describe_object(option.get("affectedId") or option.get("affectorId"))
        cost = format_mana_cost(option.get("manaCost", []))
        label = f"{option_type} {target}" if target != "Unknown" else option_type
        return f"{label} for {cost}" if cost else label

    def attacker_target_name(recipient):
        if recipient.get("type") == "DamageRecType_Player":
            return state.get_player_name(recipient.get("playerSystemSeatId"))
        return describe_object(recipient.get("damageRecipientInstanceId"))

    for event_type, data in events:
        if event_type == "match_state":
            mse = data["matchGameRoomStateChangedEvent"]
            gri = mse["gameRoomInfo"]
            config = gri.get("gameRoomConfig", {})
            match_id = config.get("matchId")

            # New match starting
            if gri.get("stateType") == "MatchGameRoomStateType_Playing":
                saved_seat = state.our_seat
                state = GameState()
                state.our_seat = saved_seat  # Preserve seat from ConnectResp
                state.match_id = match_id
                in_match = True
                for rp in config.get("reservedPlayers", []):
                    seat = rp["systemSeatId"]
                    state.player_names[seat] = rp["playerName"]
                    state.event_id = rp.get("eventId")
                players = gri.get("players", [])
                for p in players:
                    state.player_names[p["systemSeatId"]] = p["playerName"]
                prev_turn = None
                prev_phase = None
                prev_step = None
                pending_turn_header = None
                pending_phase_header = None
                player_turn_counts = {}

                names = [state.player_names.get(s, "?") for s in sorted(state.player_names)]
                out(f"\n{'='*60}")
                out(f"=== Match: {' vs '.join(names)} ===")
                if state.event_id:
                    out(f"Event: {state.event_id}")
                out(f"Match ID: {match_id}")
                out(f"{'='*60}")

            # Match completed
            if gri.get("stateType") == "MatchGameRoomStateType_MatchCompleted":
                if not in_match:
                    continue
                # Also pick up player names from reservedPlayers if not already set
                if not state.player_names:
                    for rp in config.get("reservedPlayers", []):
                        state.player_names[rp["systemSeatId"]] = rp["playerName"]
                # Build team→seat mapping from reservedPlayers
                team_to_seat = {}
                for rp in config.get("reservedPlayers", []):
                    team_to_seat[rp.get("teamId")] = rp["systemSeatId"]

                fmr = gri.get("finalMatchResult", {})
                results = fmr.get("resultList", [])
                match_result_shown = False
                # Collect per-game concession/timeout info
                game_reasons = {}  # scope -> (loser_seat, reason)
                for r in results:
                    scope = r.get("scope", "")
                    reason = r.get("reason", "")
                    if "Game" in scope and reason:
                        winner_team = r.get("winningTeamId")
                        # Find the loser (the other player)
                        for tid, sid in team_to_seat.items():
                            if tid != winner_team:
                                game_reasons[scope] = (sid, reason)
                for r in results:
                    scope = r.get("scope", "")
                    result = r.get("result", "")
                    reason = r.get("reason", "")
                    if "Match" in scope and not match_result_shown:
                        match_result_shown = True
                        # Show per-game reasons (concession, timeout)
                        for gs, (loser_seat, gr) in game_reasons.items():
                            loser_name = state.get_player_name(loser_seat)
                            if "Concede" in gr:
                                out(f"    {loser_name} concedes")
                            elif "Timeout" in gr:
                                out(f"    {loser_name} times out")
                        if "Draw" in result:
                            out(f"\n=== Result: Draw ({reason.replace('ResultReason_', '')}) ===")
                        elif "WinLoss" in result:
                            winner_team = r.get("winningTeamId")
                            winner_seat = team_to_seat.get(winner_team)
                            winner_name = state.get_player_name(winner_seat) if winner_seat else "Unknown"
                            out(f"\n=== Result: {winner_name} wins ===")
                in_match = False
                out("")

        elif event_type == "gre":
            gre_data = data.get("greToClientEvent", {})
            msgs = gre_data.get("greToClientMessages", [])
            for msg in msgs:
                msg_type = msg.get("type", "")

                # ConnectResp tells us our seat (may arrive before Playing state)
                if msg_type == "GREMessageType_ConnectResp":
                    seats = msg.get("systemSeatIds", [])
                    if seats:
                        state.our_seat = seats[0]
                    continue

                if not in_match:
                    continue

                # Feature 8: Die roll result
                if msg_type == "GREMessageType_DieRollResultsResp":
                    if not state.die_roll_shown:
                        state.die_roll_shown = True
                        drr = msg.get("dieRollResultsResp", {})
                        rolls = drr.get("playerDieRolls", [])
                        if len(rolls) >= 2:
                            best = max(rolls, key=lambda r: r.get("rollValue", 0))
                            winner_seat = best.get("systemSeatId")
                            winner_name = state.get_player_name(winner_seat)
                            roll_strs = [f"{state.get_player_name(r['systemSeatId'])} rolled {r['rollValue']}" for r in rolls]
                            out(f"  Die roll: {', '.join(roll_strs)}")
                            out(f"  {winner_name} wins the roll and goes first")
                    continue

                # Cache ActionsAvailableReq for feature 7
                if msg_type == "GREMessageType_ActionsAvailableReq":
                    actions = msg.get("actionsAvailableReq", {}).get("actions", [])
                    state.pending_actions = actions
                    continue

                # Cache SelectTargetsReq for feature 2
                if msg_type == "GREMessageType_SelectTargetsReq":
                    msg_id = msg.get("msgId")
                    state.pending_select_targets[msg_id] = msg
                    continue

                # Feature 3: Blocking assignments — buffer, only show final state
                if msg_type == "GREMessageType_DeclareBlockersReq":
                    dbr = msg.get("declareBlockersReq", {})
                    assignments = []
                    for blocker in dbr.get("blockers", []):
                        selected = blocker.get("selectedAttackerInstanceIds", [])
                        if selected:
                            blocker_name = state.get_card_name(blocker["blockerInstanceId"], card_db)
                            for atk_id in selected:
                                atk_name = state.get_card_name(atk_id, card_db)
                                assignments.append(f"    {blocker_name} blocks {atk_name}")
                    if assignments:
                        state.pending_blockers = assignments
                    continue

                # Cache SelectNReq for feature 5
                if msg_type == "GREMessageType_SelectNReq":
                    msg_id = msg.get("msgId")
                    state.pending_select_n[msg_id] = msg
                    continue

                if msg_type == "GREMessageType_ChooseStartingPlayerReq":
                    chooser = msg_player_name(msg)
                    options = [state.get_player_name(s) for s in msg.get("chooseStartingPlayerReq", {}).get("systemSeatIds", [])]
                    flush_headers()
                    out(f"    {chooser}: Choose starting player ({', '.join(options)})")
                    continue

                if msg_type == "GREMessageType_MulliganReq":
                    prompt = msg.get("prompt", {})
                    cards = next((p.get("numberValue") for p in prompt.get("parameters", [])
                                  if p.get("parameterName") == "NumberOfCards"), None)
                    player = msg_player_name(msg)
                    flush_headers()
                    out(f"    {player}: Mulligan decision" + (f" ({cards} cards)" if cards else ""))
                    continue

                if msg_type == "GREMessageType_PromptReq":
                    prompt = msg.get("prompt", {})
                    prompt_id = prompt.get("promptId")
                    player = prompt_player(prompt) or msg_player_name(msg)
                    if prompt_id == 37:
                        flush_headers()
                        out(f"    Priority to {player}")
                    else:
                        prompt_card = first_prompt_card(prompt)
                        detail = f" for {prompt_card}" if prompt_card else ""
                        flush_headers()
                        out(f"    {player}: Prompt {prompt_id}{detail}")
                    continue

                if msg_type == "GREMessageType_OptionalActionMessage":
                    msg_id = msg.get("msgId")
                    state.pending_optional_actions[msg_id] = msg
                    prompt_card = first_prompt_card(msg.get("prompt", {}))
                    source_id = msg.get("optionalActionMessage", {}).get("sourceId")
                    source = describe_object(source_id) if source_id else ""
                    if source in ("Unknown", "Unknown permanent", ""):
                        source = prompt_card or "optional action"
                    player = msg_player_name(msg)
                    flush_headers()
                    out(f"    {player}: Optional action from {source}")
                    continue

                if msg_type == "GREMessageType_PayCostsReq":
                    msg_id = msg.get("msgId")
                    state.pending_pay_costs[msg_id] = msg
                    player = msg_player_name(msg)
                    cost = pay_cost_label(msg)
                    source = pay_cost_source(msg)
                    flush_headers()
                    out(f"    {player}: Pay {cost or 'cost'} for {source}")
                    continue

                if msg_type == "GREMessageType_CastingTimeOptionsReq":
                    msg_id = msg.get("msgId")
                    state.pending_casting_time_options[msg_id] = msg
                    opts = msg.get("castingTimeOptionsReq", {}).get("castingTimeOptionReq", [])
                    labels = [casting_option_label(o) for o in opts]
                    player = msg_player_name(msg)
                    flush_headers()
                    out(f"    {player}: Casting options — {', '.join(labels)}")
                    continue

                if msg_type == "GREMessageType_SearchReq":
                    msg_id = msg.get("msgId")
                    state.pending_searches[msg_id] = msg
                    req = msg.get("searchReq", {})
                    player = msg_player_name(msg)
                    zones = ", ".join(state.get_zone_name(z) for z in req.get("zonesToSearch", [])) or "zones"
                    source = describe_object(req.get("sourceId")) if req.get("sourceId") else first_prompt_card(msg.get("prompt", {}))
                    max_find = req.get("maxFind")
                    count = f"up to {max_find} " if max_find else ""
                    flush_headers()
                    out(f"    {player}: Search {zones} for {count}card" + (f" ({source})" if source else ""))
                    continue

                if msg_type == "GREMessageType_OrderReq":
                    msg_id = msg.get("msgId")
                    state.pending_orders[msg_id] = msg
                    ids = msg.get("orderReq", {}).get("ids", [])
                    source = first_prompt_card(msg.get("prompt", {}))
                    player = msg_player_name(msg)
                    flush_headers()
                    out(f"    {player}: Order {len(ids)} item" + ("s" if len(ids) != 1 else "") + (f" from {source}" if source else ""))
                    continue

                if msg_type == "GREMessageType_DeclareAttackersReq":
                    msg_id = msg.get("msgId")
                    state.pending_attackers[msg_id] = msg
                    req = msg.get("declareAttackersReq", {})
                    attackers = req.get("attackers", []) or req.get("qualifiedAttackers", [])
                    names = card_list([a.get("attackerInstanceId") for a in attackers if a.get("attackerInstanceId")])
                    player = msg_player_name(msg)
                    flush_headers()
                    out(f"    {player}: Declare attackers" + (f" ({', '.join(names)})" if names else ""))
                    continue

                if msg_type == "GREMessageType_IntermissionReq":
                    req = msg.get("intermissionReq", {})
                    result = req.get("result", {})
                    winner_team = result.get("winningTeamId")
                    reason = clean_enum(result.get("reason"), "ResultReason_")
                    options = ", ".join(clean_enum(o.get("responseType"), "ClientMessageType_") for o in req.get("options", []))
                    flush_headers()
                    out(f"    Intermission: team {winner_team} wins" + (f" by {reason}" if reason else ""))
                    if options:
                        out(f"    Intermission options: {options}")
                    continue

                if msg_type in ("GREMessageType_TimerStateMessage", "GREMessageType_UIMessage", "GREMessageType_SetSettingsResp"):
                    continue

                # GameStateMessage
                if msg_type in ("GREMessageType_GameStateMessage", "GREMessageType_QueuedGameStateMessage"):
                    gsm = msg.get("gameStateMessage", {})

                    # Feature 9: Show opening hand before updating state
                    if not state.opening_hand_shown and state.our_seat and gsm.get("gameObjects"):
                        objects = gsm["gameObjects"]
                        # Find our hand zone
                        hand_zone_id = state.get_hand_zone_id(state.our_seat)
                        if hand_zone_id is None:
                            # Check zones in this message
                            for z in gsm.get("zones", []):
                                if z.get("type") == "ZoneType_Hand" and z.get("ownerSeatId") == state.our_seat:
                                    hand_zone_id = z["zoneId"]
                                    break
                        if hand_zone_id:
                            hand_cards = [o for o in objects
                                          if o.get("zoneId") == hand_zone_id
                                          and o.get("ownerSeatId") == state.our_seat]
                            if len(hand_cards) >= 5:
                                state.opening_hand_shown = True
                                card_names = [card_db.get_name(o["grpId"]) for o in hand_cards]
                                our_name = state.get_player_name(state.our_seat)
                                out(f"\n  {our_name}'s opening hand: {', '.join(card_names)}")

                    state.update_from_game_state_msg(gsm)

                    ti = gsm.get("turnInfo", state.turn_info)
                    turn_num = ti.get("turnNumber")
                    phase = ti.get("phase", "")
                    step = ti.get("step", "")
                    active = ti.get("activePlayer")

                    # Queue deferred headers — only printed when content follows
                    old_step = prev_step
                    if turn_num and turn_num != prev_turn:
                        if active:
                            player_turn_counts[active] = player_turn_counts.get(active, 0) + 1
                            pturn = player_turn_counts[active]
                            if active == state.our_seat:
                                turn_label = f"Your Turn {pturn}"
                            else:
                                active_name = state.get_player_name(active)
                                turn_label = f"{active_name}'s Turn {pturn}"
                        else:
                            turn_label = f"Turn {turn_num}"
                        pending_turn_header = turn_label
                        prev_turn = turn_num
                    if turn_num and (phase != prev_phase or step != prev_step):
                        phase_name = PHASE_NAMES.get(phase, phase)
                        step_name = STEP_NAMES.get(step, "")
                        pending_phase_header = f"{phase_name}" + (f" - {step_name}" if step_name else "")
                        prev_phase = phase
                        prev_step = step

                    # Flush buffered blockers when moving past declare blockers
                    if state.pending_blockers and step != "Step_DeclareBlock" and old_step == "Step_DeclareBlock":
                        # Output blockers under a Declare Blockers header
                        out(f"  [Combat - Declare Blockers]")
                        for bl in state.pending_blockers:
                            out(bl)
                        state.pending_blockers = []

                    # Track which seats got damage annotations (for feature 6)
                    damage_seats = set()

                    # Pre-scan annotations for object ID changes and token info
                    for ann in gsm.get("annotations", []):
                        ann_types = ann.get("type", [])
                        if "AnnotationType_ObjectIdChanged" in ann_types:
                            details = {d["key"]: d for d in ann.get("details", [])}
                            orig_id = details.get("orig_id", {}).get("valueInt32", [None])[0]
                            new_id = details.get("new_id", {}).get("valueInt32", [None])[0]
                            if orig_id and new_id:
                                state.id_changes[new_id] = orig_id
                        elif "AnnotationType_TokenCreated" in ann_types:
                            affected = ann.get("affectedIds", [])
                            affector_id = ann.get("affectorId")
                            if affected and affector_id:
                                state.token_creators[affected[0]] = affector_id

                    # Process annotations
                    for ann in gsm.get("annotations", []):
                        ann_types = ann.get("type", [])
                        ann_id = ann.get("id")

                        # Skip already-handled metadata annotations
                        if "AnnotationType_ObjectIdChanged" in ann_types:
                            continue
                        if "AnnotationType_TokenCreated" in ann_types:
                            continue

                        # Feature 1: Combat damage tracking
                        if "AnnotationType_DamageDealt" in ann_types:
                            if ann_id in state.damage_annotation_ids:
                                continue
                            state.damage_annotation_ids.add(ann_id)
                            details_list = ann.get("details", [])
                            details = {d["key"]: d for d in details_list}
                            dmg = details.get("damage", {}).get("valueInt32", [0])[0]
                            affector_id = ann.get("affectorId")
                            source_name = state.get_card_name(affector_id, card_db)
                            for target_id in ann.get("affectedIds", []):
                                if target_id in (1, 2):
                                    # Damage to player
                                    target_name = state.get_player_name(target_id)
                                    damage_seats.add(target_id)
                                else:
                                    target_name = state.get_card_name(target_id, card_db)
                                flush_headers()
                                out(f"    {source_name} deals {dmg} damage to {target_name}")
                            continue

                        # Zone transfer annotations
                        if "AnnotationType_ZoneTransfer" not in ann_types:
                            continue
                        details = {d["key"]: d for d in ann.get("details", [])}
                        category = ""
                        if "category" in details:
                            category = details["category"].get("valueString", [""])[0]
                        affected = ann.get("affectedIds", [])
                        zone_src = details.get("zone_src", {}).get("valueInt32", [0])[0]
                        zone_dest = details.get("zone_dest", {}).get("valueInt32", [0])[0]

                        for inst_id in affected:
                            card_name = state.get_card_name(inst_id, card_db)
                            owner = state.get_object_owner(inst_id)
                            if owner is None:
                                src_zone = state.zones.get(zone_src, {})
                                owner = src_zone.get("ownerSeatId")
                            owner_name = state.get_player_name(owner) if owner else "Unknown"

                            verb = CATEGORY_VERBS.get(category, category)
                            src_name = state.get_zone_name(zone_src)
                            dst_name = state.get_zone_name(zone_dest)

                            if category in ("PlayLand", "CastSpell"):
                                flush_headers()
                                out(f"    {owner_name}: {verb} {card_name}")
                            elif category == "Draw":
                                flush_headers()
                                if owner == state.our_seat:
                                    out(f"    {owner_name}: Draw {card_name}")
                                else:
                                    pname = state.get_player_name(owner) if owner else "Opponent"
                                    out(f"    {pname}: Draw a card")
                            elif category == "Mill":
                                flush_headers()
                                out(f"    {owner_name}: Mill {card_name}")
                            elif category == "Resolve":
                                flush_headers()
                                out(f"    {card_name} resolves → {dst_name}")
                            elif category:
                                flush_headers()
                                out(f"    {owner_name}: {verb} {card_name} ({src_name} → {dst_name})")
                            else:
                                if src_name != dst_name:
                                    flush_headers()
                                    out(f"    {card_name}: {src_name} → {dst_name}")

                    # Feature 6: Life changes - use diff as fallback for non-damage changes
                    if gsm.get("players"):
                        for p in gsm["players"]:
                            seat = p.get("systemSeatNumber") or p.get("seatId")
                            life = p.get("lifeTotal")
                            if seat and life is not None:
                                old = state.players.get(seat, {}).get("lifeTotal")
                                if old is not None and old != life:
                                    # Skip if damage annotations already showed this
                                    if seat in damage_seats:
                                        continue
                                    flush_headers()
                                    name = state.get_player_name(seat)
                                    diff = life - old
                                    sign = "+" if diff > 0 else ""
                                    out(f"    {name}: Life {old} → {life} ({sign}{diff})")

        elif event_type == "client_msg":
            if not in_match:
                continue
            payload = data.get("payload", {})
            msg_type = payload.get("type", "")
            our_name = state.get_player_name(state.our_seat) if state.our_seat else "You"

            if msg_type == "ClientMessageType_MulliganResp":
                resp = payload.get("mulliganResp", {})
                decision = resp.get("decision", "")
                if decision == "MulliganOption_AcceptHand":
                    hand_size = 7 - state.mulligan_count
                    out(f"    {our_name}: Keep hand ({hand_size} cards)")
                else:
                    state.mulligan_count += 1
                    out(f"    {our_name}: Mulligan (hand {state.mulligan_count})")

            elif msg_type == "ClientMessageType_ChooseStartingPlayerResp":
                resp = payload.get("chooseStartingPlayerResp", {})
                chosen = state.get_player_name(resp.get("systemSeatId"))
                out(f"    {our_name}: Choose {chosen} to start")

            elif msg_type == "ClientMessageType_CastingTimeOptionsResp":
                resp_id = payload.get("respId")
                req = state.pending_casting_time_options.pop(resp_id, {})
                choice = payload.get("castingTimeOptionsResp", {}).get("castingTimeOptionResp", {})
                options = req.get("castingTimeOptionsReq", {}).get("castingTimeOptionReq", [])
                selected = next((o for o in options if o.get("ctoId") == choice.get("ctoId")), choice)
                out(f"    {our_name}: Choose casting option — {casting_option_label(selected)}")

            elif msg_type == "ClientMessageType_PerformAutoTapActionsResp":
                resp_id = payload.get("respId")
                req = state.pending_pay_costs.get(resp_id, {})
                paid_with = format_auto_tap(req)
                cost = pay_cost_label(req)
                source = pay_cost_source(req)
                if paid_with:
                    out(f"    {our_name}: Pay {cost or 'cost'} for {source} using {paid_with}")
                else:
                    out(f"    {our_name}: Auto-pay {cost or 'cost'} for {source}")
                state.pending_pay_costs.pop(resp_id, None)

            elif msg_type == "ClientMessageType_EffectCostResp":
                resp_id = payload.get("respId")
                req = state.pending_pay_costs.pop(resp_id, {})
                resp = payload.get("effectCostResp", {})
                selected_ids = resp.get("costSelection", {}).get("ids", [])
                selected = ", ".join(card_list(selected_ids)) if selected_ids else "selected cost"
                source = pay_cost_source(req)
                cost_type = clean_enum(resp.get("effectCostType"), "EffectCostType_")
                out(f"    {our_name}: Pay effect cost for {source}: {cost_type} {selected}".rstrip())

            elif msg_type == "ClientMessageType_SearchResp":
                resp_id = payload.get("respId")
                req = state.pending_searches.pop(resp_id, {})
                resp = payload.get("searchResp", {})
                found = card_list(resp.get("itemsFound", []))
                search_req = req.get("searchReq", {})
                zones = ", ".join(state.get_zone_name(z) for z in search_req.get("zonesToSearch", [])) or "library"
                if found:
                    out(f"    {our_name}: Search {zones} for {', '.join(found)}")
                else:
                    out(f"    {our_name}: Search {zones}, find nothing")

            elif msg_type == "ClientMessageType_OrderResp":
                resp_id = payload.get("respId")
                req = state.pending_orders.pop(resp_id, {})
                resp = payload.get("orderResp", {})
                ordered = card_list(resp.get("ids", []))
                source = first_prompt_card(req.get("prompt", {}))
                suffix = f" from {source}" if source else ""
                out(f"    {our_name}: Order{suffix}: {', '.join(ordered)}")

            elif msg_type == "ClientMessageType_DeclareAttackersResp":
                resp_id = payload.get("respId")
                req = state.pending_attackers.get(resp_id, {})
                resp = payload.get("declareAttackersResp", {})
                req_attackers = req.get("declareAttackersReq", {}).get("attackers", [])
                target = attacker_target_name(resp.get("autoDeclareDamageRecipient", {}))
                if resp.get("autoDeclare") and req_attackers:
                    names = card_list([a.get("attackerInstanceId") for a in req_attackers if a.get("attackerInstanceId")])
                    out(f"    {our_name}: Attack {target} with {', '.join(names)}")
                else:
                    out(f"    {our_name}: Declare attackers")

            elif msg_type == "ClientMessageType_SubmitAttackersReq":
                out(f"    {our_name}: Submit attackers")

            elif msg_type == "ClientMessageType_SubmitTargetsReq":
                out(f"    {our_name}: Submit targets")

            elif msg_type == "ClientMessageType_OptionalActionResp":
                resp_id = payload.get("respId")
                req = state.pending_optional_actions.pop(resp_id, {})
                response = payload.get("optionalResp", {}).get("response", "")
                source_id = req.get("optionalActionMessage", {}).get("sourceId")
                source = describe_object(source_id) if source_id else ""
                if source in ("Unknown", "Unknown permanent", ""):
                    source = first_prompt_card(req.get("prompt", {})) or "optional action"
                out(f"    {our_name}: {response_word(response)} optional action from {source}")

            elif msg_type == "ClientMessageType_DrawCardResp":
                out(f"    {our_name}: Choose to draw a card")

            elif msg_type == "ClientMessageType_RevealHandResp":
                out(f"    {our_name}: Choose to reveal hand")

            elif msg_type == "ClientMessageType_GroupResp":
                resp = payload.get("groupResp", {})
                ids = resp.get("ids", []) or resp.get("groupIds", [])
                detail = f": {', '.join(card_list(ids))}" if ids else ""
                out(f"    {our_name}: Choose group{detail}")

            elif msg_type == "ClientMessageType_AssignDamageResp":
                resp = payload.get("assignDamageResp", {})
                assignments = resp.get("damageAssignments", []) or resp.get("assignments", [])
                if assignments:
                    parts = []
                    for assignment in assignments:
                        target = describe_object(assignment.get("targetId") or assignment.get("targetInstanceId"))
                        amount = assignment.get("damage") or assignment.get("amount")
                        parts.append(f"{amount} to {target}" if amount is not None else target)
                    out(f"    {our_name}: Assign damage — {', '.join(parts)}")
                else:
                    out(f"    {our_name}: Assign damage")

            elif msg_type in ("ClientMessageType_SetSettingsReq", "ClientMessageType_UIMessage"):
                continue

            # Feature 7: Action display with card names
            elif msg_type == "ClientMessageType_PerformActionResp":
                resp = payload.get("performActionResp", {})
                actions = resp.get("actions", [])
                for action in actions:
                    action_type = action.get("actionType", "")
                    grp_id = action.get("grpId", 0)
                    instance_id = action.get("instanceId")
                    if action_type == "ActionType_Activate":
                        card_name = state.get_card_name(instance_id, card_db) if instance_id else card_db.get_name(grp_id)
                        out(f"    {our_name}: Activate {card_name}")
                    elif action_type == "ActionType_Special" and grp_id:
                        card_name = card_db.get_name(grp_id)
                        out(f"    {our_name}: Special action ({card_name})")

            # Feature 2: Targeting information
            elif msg_type == "ClientMessageType_SelectTargetsResp":
                resp = payload.get("selectTargetsResp", {})
                target_info = resp.get("target", {})
                targets = target_info.get("targets", [])
                for t in targets:
                    tid = t.get("targetInstanceId")
                    if tid:
                        if tid in (1, 2):
                            target_name = state.get_player_name(tid)
                        else:
                            target_name = state.get_card_name(tid, card_db)
                        out(f"    Target: {target_name}")

            # Feature 5: SelectN choices (discard, tutor, copy target, scry, ...)
            # The promptId names the choice: 1034 is "Discard two cards." —
            # the Does Machines / Jadzi discard step, not a scry. Label the
            # selection with the prompt text from the card database.
            elif msg_type == "ClientMessageType_SelectNResp":
                resp_id = payload.get("respId")
                req = state.pending_select_n.pop(resp_id, {})
                prompt = req.get("prompt", {})
                select_req = req.get("selectNReq", {})
                selected_ids = payload.get("selectNResp", {}).get("ids", [])
                if not req or not selected_ids:
                    continue
                source_id = select_req.get("sourceId")
                if not source_id:
                    for p in prompt.get("parameters", []):
                        if p.get("parameterName") == "CardId":
                            source_id = p.get("numberValue")
                            break
                source_name = state.get_card_name(source_id, card_db) if source_id else "effect"
                label = card_db.get_prompt_text(prompt.get("promptId")).rstrip(".") or "Select"
                if select_req.get("idType") == "IdType_InstanceId":
                    chosen = ", ".join(state.get_card_name(sid, card_db) for sid in selected_ids)
                else:
                    # Static option lists carry option ids, not card instances
                    chosen = ", ".join(f"option {sid}" for sid in selected_ids)
                out(f"    {our_name}: {label} ({source_name}): {chosen}")

    return output_lines


def process_matches(log_path):
    """Process all matches in a log file and print play-by-play."""
    for line in render_events(parse_log(log_path)):
        print(line)


def main():
    parser = argparse.ArgumentParser(
        description="Parse MTGA Player.log into a play-by-play narrative"
    )
    parser.add_argument(
        "log_file",
        nargs="?",
        help="Path to Player.log (default: auto-detect)",
    )
    args = parser.parse_args()

    log_file = find_player_log(args.log_file)
    if not log_file or not os.path.exists(log_file):
        print(f"Error: Log file not found: {log_file or args.log_file}", file=sys.stderr)
        sys.exit(1)

    process_matches(log_file)


if __name__ == "__main__":
    main()
