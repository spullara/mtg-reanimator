"""Shared MTGA game state library for real-time tracking.

Provides CardDatabase (card name lookups) and GameState (incremental game state
tracking from GRE messages). Extracted from parse_game.py for reuse by
live_server.py.
"""

import sqlite3

from paths import find_card_db

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


class CardDatabase:
    """Look up card names from the MTGA CardDatabase SQLite file."""

    def __init__(self, db_path=None):
        self._cache = {}
        found = find_card_db(db_path)
        self._db_path = str(found) if found else None

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

    def get_card_info(self, grp_id):
        """Return dict with name and any extra info available for a grpId."""
        name = self.get_name(grp_id)
        return {"grpId": grp_id, "name": name}


class GameState:
    """Track evolving game state from GRE messages."""

    def __init__(self, card_db=None):
        self.card_db = card_db or CardDatabase()
        self.zones = {}
        self.objects = {}
        self.players = {}
        self.turn_info = {}
        self.match_id = None
        self.player_names = {}
        self.our_seat = None
        self.event_id = None
        self.token_creators = {}
        self.id_changes = {}
        self.deleted_objects = {}
        self.pending_actions = []
        self.log = []
        self.status = "waiting"
        self._changed = False

    def _resolve_object(self, instance_id):
        obj = self.objects.get(instance_id) or self.deleted_objects.get(instance_id)
        if obj:
            return obj
        orig_id = self.id_changes.get(instance_id)
        if orig_id:
            return self.objects.get(orig_id) or self.deleted_objects.get(orig_id)
        return None

    def get_card_name(self, instance_id):
        obj = self._resolve_object(instance_id)
        if obj:
            return self.card_db.get_name(obj.get("grpId", 0))
        if instance_id in self.token_creators:
            creator_obj = self._resolve_object(self.token_creators[instance_id])
            if creator_obj:
                cname = self.card_db.get_name(creator_obj.get("grpId", 0))
                if cname not in ("Unknown",):
                    return f"token (from {cname})"
            return "token"
        return "Unknown"

    def get_player_name(self, seat_id):
        return self.player_names.get(seat_id, f"Player#{seat_id}")

    def _update_game_state_msg(self, gsm):
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
        for did in gsm.get("diffDeletedInstanceIds", []):
            obj = self.objects.pop(did, None)
            if obj:
                self.deleted_objects[did] = obj
        for ann in gsm.get("annotations", []):
            ann_types = ann.get("type", [])
            if "AnnotationType_ObjectIdChanged" in ann_types:
                details = {d["key"]: d for d in ann.get("details", [])}
                orig_id = details.get("orig_id", {}).get("valueInt32", [None])[0]
                new_id = details.get("new_id", {}).get("valueInt32", [None])[0]
                if orig_id and new_id:
                    self.id_changes[new_id] = orig_id
            elif "AnnotationType_TokenCreated" in ann_types:
                affected = ann.get("affectedIds", [])
                affector_id = ann.get("affectorId")
                if affected and affector_id:
                    self.token_creators[affected[0]] = affector_id

    def update(self, msg_dict):
        """Accept a single parsed GRE/match message dict and update state.

        Handles: greToClientEvent, matchGameRoomStateChangedEvent,
        and clientToGreMessage (for action tracking).
        Returns True if state changed.
        """
        self._changed = False

        if "matchGameRoomStateChangedEvent" in msg_dict:
            mse = msg_dict["matchGameRoomStateChangedEvent"]
            gri = mse.get("gameRoomInfo", {})
            config = gri.get("gameRoomConfig", {})

            if gri.get("stateType") == "MatchGameRoomStateType_Playing":
                self.status = "in_game"
                self.match_id = config.get("matchId")
                for rp in config.get("reservedPlayers", []):
                    seat = rp["systemSeatId"]
                    self.player_names[seat] = rp["playerName"]
                    self.event_id = rp.get("eventId")
                for p in gri.get("players", []):
                    self.player_names[p["systemSeatId"]] = p["playerName"]
                self._changed = True

            elif gri.get("stateType") == "MatchGameRoomStateType_MatchCompleted":
                self.status = "match_complete"
                self._changed = True

        if "greToClientEvent" in msg_dict:
            gre = msg_dict["greToClientEvent"]
            for gre_msg in gre.get("greToClientMessages", []):
                msg_type = gre_msg.get("type", "")

                if msg_type == "GREMessageType_ConnectResp":
                    seats = gre_msg.get("systemSeatIds", [])
                    if seats:
                        self.our_seat = seats[0]
                    self._changed = True

                elif msg_type == "GREMessageType_GameStateMessage":
                    gsm = gre_msg.get("gameStateMessage", {})
                    self._update_game_state_msg(gsm)
                    self.status = "in_game"
                    self._changed = True

                elif msg_type == "GREMessageType_ActionsAvailableReq":
                    actions = gre_msg.get("actionsAvailableReq", {}).get("actions", [])
                    self.pending_actions = actions
                    self._changed = True

        if "clientToGreMessage" in msg_dict:
            self._changed = True

        return self._changed

    def _get_objects_in_zone_type(self, zone_type, seat_id=None):
        """Get objects in zones of a given type, optionally filtered by seat."""
        zone_ids = set()
        for zid, z in self.zones.items():
            if z.get("type") == zone_type:
                if seat_id is None or z.get("ownerSeatId") == seat_id:
                    zone_ids.add(zid)
        result = []
        for obj in self.objects.values():
            if obj.get("zoneId") in zone_ids:
                result.append(obj)
        return result

    def _obj_to_dict(self, obj):
        """Convert a game object to a serializable dict."""
        grp_id = obj.get("grpId", 0)
        name = self.card_db.get_name(grp_id)
        d = {
            "instanceId": obj.get("instanceId"),
            "name": name,
            "grpId": grp_id,
        }
        card_types = obj.get("cardTypes", [])
        if card_types:
            d["cardTypes"] = card_types
        mana_cost = obj.get("manaCost")
        if mana_cost:
            d["manaCost"] = mana_cost
        if "power" in obj:
            d["power"] = obj["power"].get("value") if isinstance(obj["power"], dict) else obj["power"]
        if "toughness" in obj:
            d["toughness"] = obj["toughness"].get("value") if isinstance(obj["toughness"], dict) else obj["toughness"]
        if "isTapped" in obj:
            d["tapped"] = obj["isTapped"]
        return d

    def _player_dict(self, seat_id, is_opponent=False):
        """Build the player state dict for a given seat."""
        p = self.players.get(seat_id, {})
        name = self.get_player_name(seat_id)
        life = p.get("lifeTotal", 20)

        battlefield = [self._obj_to_dict(o) for o in self._get_objects_in_zone_type("ZoneType_Battlefield", seat_id)]
        graveyard = [self._obj_to_dict(o) for o in self._get_objects_in_zone_type("ZoneType_Graveyard", seat_id)]
        exile = [self._obj_to_dict(o) for o in self._get_objects_in_zone_type("ZoneType_Exile", seat_id)]
        library_count = len(self._get_objects_in_zone_type("ZoneType_Library", seat_id))

        d = {
            "name": name,
            "seatId": seat_id,
            "life": life,
            "battlefield": battlefield,
            "graveyard": graveyard,
            "exile": exile,
            "libraryCount": library_count,
        }

        if is_opponent:
            d["handCount"] = len(self._get_objects_in_zone_type("ZoneType_Hand", seat_id))
        else:
            hand = [self._obj_to_dict(o) for o in self._get_objects_in_zone_type("ZoneType_Hand", seat_id)]
            d["hand"] = hand

        return d

    def to_dict(self, our_seat_id=None):
        """Return full game state as a serializable dict."""
        seat = our_seat_id or self.our_seat

        # Build match info
        match_players = []
        for s in sorted(self.player_names):
            match_players.append({"name": self.player_names[s], "seatId": s})

        # Turn info
        turn = {}
        if self.turn_info:
            active = self.turn_info.get("activePlayer")
            turn = {
                "number": self.turn_info.get("turnNumber"),
                "activePlayer": self.get_player_name(active) if active else None,
                "phase": PHASE_NAMES.get(self.turn_info.get("phase", ""), self.turn_info.get("phase", "")),
                "step": STEP_NAMES.get(self.turn_info.get("step", ""), self.turn_info.get("step", "")),
            }

        # Opponent seat
        opp_seat = None
        for s in self.player_names:
            if s != seat:
                opp_seat = s
                break

        # Stack objects
        stack_objs = self._get_objects_in_zone_type("ZoneType_Stack")
        stack = []
        for obj in stack_objs:
            sd = self._obj_to_dict(obj)
            controller = obj.get("controllerSeatId") or obj.get("ownerSeatId")
            sd["controller"] = self.get_player_name(controller) if controller else "Unknown"
            stack.append(sd)

        # Actions available
        actions_avail = []
        for a in self.pending_actions:
            grp_id = a.get("grpId", 0)
            actions_avail.append({
                "actionIndex": a.get("actionIndex", 0),
                "actionType": a.get("actionType", ""),
                "name": self.card_db.get_name(grp_id),
                "grpId": grp_id,
            })

        result = {
            "status": self.status,
            "match": {
                "players": match_players,
                "matchId": self.match_id,
            },
            "turn": turn,
            "you": self._player_dict(seat) if seat else {},
            "opponent": self._player_dict(opp_seat, is_opponent=True) if opp_seat else {},
            "stack": stack,
            "actionsAvailable": actions_avail,
            "log": self.log[-50:],
        }
        return result
