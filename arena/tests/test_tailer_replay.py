"""E2E test: replay a saved Player.log through the tailer at speed.

Checks that the tailer produces the same events, the same final dashboard
state, and the same advisor triggers as the offline parser (parse_game).
Also covers partial lines, in-place truncation, and rotation to
Player-prev.log.

Set MTGA_SAMPLE_LOG to a real Player.log to replay it; otherwise a
bundled synthetic log is used. No network or API calls are made.
Writes a JSON report artifact and prints its path.
"""

import hashlib
import json
import os
import random
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from log_tailer import LogEventParser, PlayerLogTailer  # noqa: E402
from live_server import has_actions_available_req  # noqa: E402
from mtga_state import GameState  # noqa: E402
from parse_game import parse_log  # noqa: E402

SAMPLE_LOG = os.environ.get("MTGA_SAMPLE_LOG")


class StubCardDB:
    """Deterministic card names: no MTGA install needed."""

    def get_name(self, grp_id):
        return f"#{grp_id}"


def synthetic_log_text():
    """A small Player.log lookalike covering every event type."""
    gre = lambda msgs: json.dumps(  # noqa: E731
        {"greToClientEvent": {"greToClientMessages": msgs}}
    )
    match_state = json.dumps({
        "matchGameRoomStateChangedEvent": {
            "gameRoomInfo": {
                "stateType": "MatchGameRoomStateType_Playing",
                "gameRoomConfig": {
                    "matchId": "match-1",
                    "reservedPlayers": [
                        {"systemSeatId": 1, "playerName": "You", "eventId": "Play"},
                        {"systemSeatId": 2, "playerName": "Opp", "eventId": "Play"},
                    ],
                },
            }
        }
    })
    gsm = gre([{
        "type": "GREMessageType_GameStateMessage",
        "gameStateMessage": {
            "zones": [
                {"zoneId": 31, "type": "ZoneType_Hand", "ownerSeatId": 1},
                {"zoneId": 28, "type": "ZoneType_Battlefield"},
            ],
            "gameObjects": [
                {"instanceId": 100, "grpId": 5555, "zoneId": 31},
                {"instanceId": 101, "grpId": 6666, "zoneId": 28},
            ],
            "players": [{"systemSeatNumber": 1, "lifeTotal": 20}],
            "turnInfo": {"turnNumber": 3, "activePlayer": 1,
                         "phase": "Phase_Main1"},
        },
    }])
    connect = gre([{"type": "GREMessageType_ConnectResp", "systemSeatIds": [1]}])
    actions = gre([{
        "type": "GREMessageType_ActionsAvailableReq",
        "actionsAvailableReq": {"actions": [
            {"actionIndex": 0, "actionType": "ActionType_Cast", "grpId": 5555},
            {"actionIndex": 1, "actionType": "ActionType_Pass", "grpId": 0},
        ]},
    }])
    done = json.dumps({
        "matchGameRoomStateChangedEvent": {
            "gameRoomInfo": {"stateType": "MatchGameRoomStateType_MatchCompleted"}
        }
    })
    client_payload = json.dumps(
        {"payload": {"type": "ClientMessageType_PerformActionResp"}}, indent=2
    )
    return "\n".join([
        "[UnityCrossThreadLogger]4/7/2026 9:30:15 PM: Match to game",
        match_state,
        "noise line that is not JSON",
        connect,
        gsm,
        "[UnityCrossThreadLogger]==> ClientToGremessage",
        client_payload,
        actions,
        "{ \"transactionId\": \"ignored, no known event\" }",
        done,
        "",
    ])


def source_log_path(tmp_dir):
    if SAMPLE_LOG and Path(SAMPLE_LOG).is_file():
        return Path(SAMPLE_LOG)
    p = tmp_dir / "synthetic_player.log"
    p.write_text(synthetic_log_text(), encoding="utf-8")
    return p


def final_state(events):
    state = GameState(card_db=StubCardDB())
    for _etype, data in events:
        state.update(data)
    return state.to_dict()


class TailerReplayTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp_dir = Path(self._tmp.name)
        self.log_path = self.tmp_dir / "Player.log"

    def tearDown(self):
        self._tmp.cleanup()

    def test_replay_matches_offline_parser(self):
        """Grow the file in odd-sized chunks; tailer must match parse_log."""
        source = source_log_path(self.tmp_dir)
        raw = source.read_bytes()

        tailer = PlayerLogTailer(self.log_path, from_start=True)
        tailed_events = []
        rng = random.Random(42)
        pos = 0
        with open(self.log_path, "wb") as out:
            while pos < len(raw):
                size = rng.choice([1, 7, 100, 999, 4096, 70000])
                out.write(raw[pos:pos + size])
                out.flush()
                pos += size
                tailed_events.extend(tailer.poll())
        tailed_events.extend(tailer.poll())

        offline_events = list(parse_log(self.log_path))
        self.assertEqual(len(tailed_events), len(offline_events))
        self.assertEqual(tailed_events, offline_events)

        tailed_state = final_state(tailed_events)
        offline_state = final_state(offline_events)
        self.assertEqual(tailed_state, offline_state)

        tailed_triggers = sum(
            1 for _t, d in tailed_events if has_actions_available_req(d)
        )
        offline_triggers = sum(
            1 for _t, d in offline_events if has_actions_available_req(d)
        )
        self.assertEqual(tailed_triggers, offline_triggers)
        self.assertGreater(tailed_triggers, 0)

        report = {
            "source_log": str(source),
            "events": len(tailed_events),
            "events_by_type": {
                t: sum(1 for et, _ in tailed_events if et == t)
                for t in ("gre", "match_state", "client_msg")
            },
            "advisor_triggers": tailed_triggers,
            "final_state_sha256": hashlib.sha256(
                json.dumps(tailed_state, sort_keys=True).encode()
            ).hexdigest(),
            "matches_offline_parser": True,
        }
        report_path = Path(tempfile.gettempdir()) / "tailer_replay_report.json"
        report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(f"\n[artifact] {report_path}\n{json.dumps(report, indent=2)}")

    def test_parser_handles_byte_by_byte_feed(self):
        """Partial lines: feeding one character at a time changes nothing."""
        source = source_log_path(self.tmp_dir)
        text = source.read_text(encoding="utf-8", errors="replace")
        parser = LogEventParser()
        events = []
        step = 1 if len(text) < 100_000 else 1_000
        for i in range(0, len(text), step):
            events.extend(parser.feed(text[i:i + step]))
        self.assertEqual(events, list(parse_log(source)))

    def test_truncation_in_place(self):
        line = json.dumps({"greToClientEvent": {"greToClientMessages": [
            {"type": "GREMessageType_ConnectResp", "systemSeatIds": [1],
             "padding": "x" * 64}
        ]}})
        self.log_path.write_text(line + "\n", encoding="utf-8")
        tailer = PlayerLogTailer(self.log_path, from_start=True)
        self.assertEqual(len(tailer.poll()), 1)

        line2 = json.dumps({"greToClientEvent": {"greToClientMessages": [
            {"type": "GREMessageType_ConnectResp", "systemSeatIds": [2]}
        ]}})
        self.log_path.write_text(line2 + "\n", encoding="utf-8")  # truncates
        events = tailer.poll()
        self.assertEqual(len(events), 1)
        self.assertEqual(
            events[0][1]["greToClientEvent"]["greToClientMessages"][0]
            ["systemSeatIds"], [2],
        )

    def test_rotation_to_player_prev(self):
        def gre_line(seat):
            return json.dumps({"greToClientEvent": {"greToClientMessages": [
                {"type": "GREMessageType_ConnectResp", "systemSeatIds": [seat]}
            ]}}) + "\n"

        with open(self.log_path, "w", encoding="utf-8") as f:
            f.write(gre_line(1))
        tailer = PlayerLogTailer(self.log_path, from_start=True)
        self.assertEqual(len(tailer.poll()), 1)

        # Arena appends final events, rotates, then starts a fresh log.
        with open(self.log_path, "a", encoding="utf-8") as f:
            f.write(gre_line(2))
        os.replace(self.log_path, self.tmp_dir / "Player-prev.log")
        with open(self.log_path, "w", encoding="utf-8") as f:
            f.write(gre_line(3))

        events = tailer.poll()
        seats = [
            e[1]["greToClientEvent"]["greToClientMessages"][0]["systemSeatIds"][0]
            for e in events
        ]
        self.assertEqual(seats, [2, 3])

    def test_default_start_is_end_of_file(self):
        def gre_line(seat):
            return json.dumps({"greToClientEvent": {"greToClientMessages": [
                {"type": "GREMessageType_ConnectResp", "systemSeatIds": [seat]}
            ]}}) + "\n"

        self.log_path.write_text(gre_line(1), encoding="utf-8")
        tailer = PlayerLogTailer(self.log_path)  # from_start=False
        self.assertEqual(tailer.poll(), [])
        with open(self.log_path, "a", encoding="utf-8") as f:
            f.write(gre_line(2))
        events = tailer.poll()
        self.assertEqual(len(events), 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
