#!/usr/bin/env python3
"""Archive every finished MTGA match from Player.log.

Arena overwrites Player.log at each start, so at most two sessions survive
on disk (Player.log and Player-prev.log). This collector saves each
finished match to its own JSON file: match metadata, the parsed
play-by-play, and the raw GRE log lines for that match. The raw lines can
be written back to a file and re-parsed with parse_game.

Run while Arena runs (tails the log, saves each match as it ends):

    python -m arena.collect

Archive the matches already in Player.log and Player-prev.log, then exit:

    python -m arena.collect --once

Captures go to arena/captures/ by default. That folder is git-ignored:
captures hold account and session data and must never be committed.
Override the folder with --dir or the ARENA_CAPTURES_DIR environment
variable. Works on Windows and macOS.
"""

import argparse
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# arena/__init__.py puts this folder on sys.path, so flat imports work both
# as a script and as `python -m arena.collect`.
from log_tailer import PlayerLogTailer
from parse_game import CardDatabase, render_events
from paths import find_player_log, prev_player_log

DEFAULT_CAPTURES_DIR = Path(__file__).resolve().parent / "captures"


def captures_dir(explicit=None):
    """Resolve the captures folder: --dir, then env, then arena/captures/."""
    if explicit:
        return Path(explicit)
    env = os.environ.get("ARENA_CAPTURES_DIR")
    if env:
        return Path(env)
    return DEFAULT_CAPTURES_DIR


class MatchSlicer:
    """Feed Player.log text; return each finished match with its raw lines.

    Implements the parser interface PlayerLogTailer expects: feed(text)
    returns a list, reset() drops partial-line state. Here feed() returns
    finished matches: {"match_id", "raw_lines", "events"}. Lines that do
    not belong to a known event type are dropped; everything between the
    end of one match and the end of the next (ConnectResp, deck submit,
    the match itself) is kept with that next match.
    """

    def __init__(self):
        self._tail = ""
        self._accum_raw = None   # raw lines of a multi-line client message
        self._accum_json = None  # JSON lines only (header excluded)
        self._depth = 0
        self._lines = []         # raw event lines since the last finished match
        self._events = []        # parsed (event_type, data), same span
        self._match_id = None    # from the most recent Playing state

    def feed(self, text):
        """Consume a text chunk and return the list of finished matches."""
        finished = []
        self._tail += text
        while True:
            nl = self._tail.find("\n")
            if nl < 0:
                break
            line = self._tail[:nl].rstrip("\r")
            self._tail = self._tail[nl + 1:]
            match = self._feed_line(line)
            if match:
                finished.append(match)
        return finished

    def reset(self):
        """Drop partial-line state (file replaced); keep the open match."""
        self._tail = ""
        self._accum_raw = None
        self._accum_json = None
        self._depth = 0

    def _feed_line(self, line):
        if self._accum_raw is not None:
            self._accum_raw.append(line)
            self._accum_json.append(line)
            self._depth += line.count("{") - line.count("}")
            if self._depth <= 0:
                raw, blob = self._accum_raw, "\n".join(self._accum_json)
                self._accum_raw = None
                self._accum_json = None
                try:
                    data = json.loads(blob)
                except json.JSONDecodeError:
                    return None
                self._keep(raw, ("client_msg", data))
            return None

        if line.startswith("{ ") or line.startswith('{"'):
            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                return None
            if "greToClientEvent" in data:
                self._keep([line], ("gre", data))
                return None
            if "matchGameRoomStateChangedEvent" in data:
                return self._match_state(line, data)
            return None

        if "ClientToGremessage" in line or "ClientToGreuimessage" in line:
            self._accum_raw = [line]
            self._accum_json = []
            self._depth = 0
        return None

    def _keep(self, raw_lines, event):
        self._lines.extend(raw_lines)
        self._events.append(event)

    def _match_state(self, line, data):
        gri = data["matchGameRoomStateChangedEvent"].get("gameRoomInfo", {})
        config = gri.get("gameRoomConfig", {})
        state_type = gri.get("stateType")
        if state_type == "MatchGameRoomStateType_Playing":
            self._match_id = config.get("matchId") or self._match_id
        self._keep([line], ("match_state", data))
        if state_type != "MatchGameRoomStateType_MatchCompleted":
            return None
        match_id = config.get("matchId") or self._match_id
        match = {
            "match_id": match_id,
            "raw_lines": self._lines,
            "events": self._events,
        }
        self._lines = []
        self._events = []
        self._match_id = None
        return match if match_id else None


def match_metadata(events, card_db):
    """Pull players, seat, deck, and result out of one match's events."""
    meta = {
        "players": [],
        "our_seat": None,
        "event_id": None,
        "deck": {},
        "result": {},
        "turns": None,
    }
    names = {}
    team_to_seat = {}

    for event_type, data in events:
        if event_type == "gre":
            for msg in data.get("greToClientEvent", {}).get("greToClientMessages", []):
                msg_type = msg.get("type")
                if msg_type == "GREMessageType_ConnectResp":
                    seats = msg.get("systemSeatIds", [])
                    if seats:
                        meta["our_seat"] = seats[0]
                    deck_cards = (
                        msg.get("connectResp", {})
                        .get("deckMessage", {})
                        .get("deckCards")
                    )
                    if deck_cards:
                        deck = {}
                        for grp_id in deck_cards:
                            name = card_db.get_name(grp_id)
                            deck[name] = deck.get(name, 0) + 1
                        meta["deck"] = deck
                elif msg_type in ("GREMessageType_GameStateMessage",
                                  "GREMessageType_QueuedGameStateMessage"):
                    turn = msg.get("gameStateMessage", {}).get("turnInfo", {}).get("turnNumber")
                    if turn:
                        meta["turns"] = turn
        elif event_type == "match_state":
            gri = data["matchGameRoomStateChangedEvent"].get("gameRoomInfo", {})
            config = gri.get("gameRoomConfig", {})
            for rp in config.get("reservedPlayers", []):
                names[rp["systemSeatId"]] = rp["playerName"]
                team_to_seat[rp.get("teamId")] = rp["systemSeatId"]
                meta["event_id"] = rp.get("eventId") or meta["event_id"]
            for p in gri.get("players", []):
                names[p["systemSeatId"]] = p["playerName"]
            fmr = gri.get("finalMatchResult")
            if fmr:
                meta["result"] = _final_result(fmr, team_to_seat, names)

    meta["players"] = [
        {"seat": seat, "name": names[seat]} for seat in sorted(names)
    ]
    if meta["result"].get("winner_seat") is not None and meta["our_seat"] is not None:
        meta["result"]["won"] = meta["result"]["winner_seat"] == meta["our_seat"]
    return meta


def _final_result(fmr, team_to_seat, names):
    result = {}
    for r in fmr.get("resultList", []):
        scope = r.get("scope", "")
        if "Match" in scope and "winner_seat" not in result:
            winner_seat = team_to_seat.get(r.get("winningTeamId"))
            result["winner_seat"] = winner_seat
            result["winner"] = names.get(winner_seat)
            result["kind"] = "draw" if "Draw" in r.get("result", "") else "win_loss"
        elif "Game" in scope and r.get("reason"):
            result.setdefault("reasons", []).append(
                r["reason"].replace("ResultReason_", "")
            )
    return result


def save_match(match, out_dir, card_db, quiet=False):
    """Write one match to out_dir; return the path, or None if it exists."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / f"match_{match['match_id']}.json"
    if path.exists():
        return None
    doc = {
        "match_id": match["match_id"],
        "saved_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        **match_metadata(match["events"], card_db),
        "play_by_play": render_events(match["events"], card_db),
        "raw_lines": match["raw_lines"],
    }
    tmp = path.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(doc, indent=1), encoding="utf-8")
    os.replace(tmp, path)
    if not quiet:
        print(f"Saved match {match['match_id']} -> {path}")
    return path


def archive_logs(log_paths, out_dir, card_db=None, quiet=False):
    """One-shot: archive every finished match found in the given log files."""
    card_db = card_db or CardDatabase()
    saved = []
    for log_path in log_paths:
        log_path = Path(log_path)
        if not log_path.is_file():
            continue
        slicer = MatchSlicer()
        text = log_path.read_text(encoding="utf-8", errors="replace")
        if not text.endswith("\n"):
            text += "\n"
        for match in slicer.feed(text):
            path = save_match(match, out_dir, card_db, quiet=quiet)
            if path:
                saved.append(path)
    return saved


def watch(log_path, out_dir, card_db=None, poll_seconds=2.0, quiet=False):
    """Live mode: follow Player.log and save each match when it ends.

    Starts at the top of the current Player.log, so matches already played
    this session are archived too (duplicates are skipped by match ID).
    """
    card_db = card_db or CardDatabase()
    tailer = PlayerLogTailer(log_path, from_start=True, parser=MatchSlicer())
    if not quiet:
        print(f"Watching {log_path} (captures -> {out_dir}). Ctrl-C to stop.")
    while True:
        for match in tailer.poll():
            save_match(match, out_dir, card_db, quiet=quiet)
        time.sleep(poll_seconds)


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Save every finished MTGA match to its own capture file"
    )
    parser.add_argument("--once", action="store_true",
                        help="archive Player.log and Player-prev.log now, then exit")
    parser.add_argument("--dir", metavar="FOLDER",
                        help="captures folder (default: arena/captures/ or ARENA_CAPTURES_DIR)")
    parser.add_argument("--log", metavar="PLAYER_LOG",
                        help="path to Player.log (default: auto-detect)")
    parser.add_argument("--card-db", metavar="DB",
                        help="path to Raw_CardDatabase_*.mtga (default: auto-detect)")
    parser.add_argument("--poll", type=float, default=2.0,
                        help="poll interval in seconds for live mode (default: 2)")
    parser.add_argument("--quiet", action="store_true", help="print nothing on save")
    args = parser.parse_args(argv)

    log_path = find_player_log(args.log)
    if not log_path or not Path(log_path).is_file():
        print(f"Error: Player.log not found: {log_path or args.log}", file=sys.stderr)
        return 1
    out_dir = captures_dir(args.dir)
    card_db = CardDatabase(args.card_db)

    if args.once:
        saved = archive_logs(
            [prev_player_log(log_path), log_path], out_dir, card_db, quiet=args.quiet
        )
        if not args.quiet:
            print(f"Archived {len(saved)} new match(es) to {out_dir}")
        return 0

    try:
        watch(log_path, out_dir, card_db, poll_seconds=args.poll, quiet=args.quiet)
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
