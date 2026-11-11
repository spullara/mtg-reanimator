"""E2E test: archive the two saved games and analyze the player's strategy.

Reads the saved Player.log / Player-prev.log from MTGA_CAPTURES_SRC
(default: the mtg-arena workspace's tools/captures). The logs are never
copied into the repo; captures go to a temp folder. The --llm path is
mocked: no network or API calls are made. Writes a JSON report artifact
and prints its path.
"""

import contextlib
import io
import json
import os
import re
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import analyze_games  # noqa: E402
from collect import MatchSlicer, archive_logs, save_match  # noqa: E402
from log_tailer import PlayerLogTailer  # noqa: E402
from parse_game import CardDatabase, render_events  # noqa: E402

SRC_DIR = Path(os.environ.get(
    "MTGA_CAPTURES_SRC",
    "/Users/sam/intent/workspaces/mtg-arena/repo/tools/captures",
))
PLAYER_LOG = SRC_DIR / "Player.log"
PREV_LOG = SRC_DIR / "Player-prev.log"
OLD_SCRY_LINE = re.compile(r"Scry \(.*\): put .* on bottom")


@unittest.skipUnless(
    PLAYER_LOG.is_file() and PREV_LOG.is_file(),
    "saved Player.log captures not available",
)
class CollectAnalyzeE2E(unittest.TestCase):
    """One flow: archive both logs once, then test every stage on them."""

    @classmethod
    def setUpClass(cls):
        cls._tmp = tempfile.TemporaryDirectory()
        cls.captures = Path(cls._tmp.name) / "captures"
        cls.card_db = CardDatabase()
        cls.saved = archive_logs(
            [PREV_LOG, PLAYER_LOG], cls.captures, cls.card_db, quiet=True
        )
        cls.docs = [
            json.loads(p.read_text(encoding="utf-8"))
            for p in sorted(cls.captures.glob("match_*.json"))
        ]

    @classmethod
    def tearDownClass(cls):
        cls._tmp.cleanup()

    def test_archives_each_match_once(self):
        self.assertEqual(len(self.saved), 2, "one match per saved log")
        ids = [d["match_id"] for d in self.docs]
        self.assertEqual(len(ids), len(set(ids)))
        # Second run: everything is already saved, by match ID
        again = archive_logs([PREV_LOG, PLAYER_LOG], self.captures, self.card_db, quiet=True)
        self.assertEqual(again, [])
        for doc in self.docs:
            self.assertTrue(doc["play_by_play"])
            self.assertTrue(doc["raw_lines"])
            self.assertEqual(len(doc["players"]), 2)
            self.assertIn("winner_seat", doc["result"])

    def test_raw_lines_replay_reproduces_play_by_play(self):
        """A capture is self-contained: its raw lines re-render identically."""
        for doc in self.docs:
            events = analyze_games.events_from_raw_lines(doc["raw_lines"])
            self.assertTrue(events)
            self.assertEqual(render_events(events, self.card_db), doc["play_by_play"])

    def test_discard_step_is_labeled_discard_not_scry(self):
        """Does Machines' discard was mislabeled 'Scry ... put X on bottom'."""
        text = "\n".join(line for doc in self.docs for line in doc["play_by_play"])
        for line in text.splitlines():
            self.assertIsNone(OLD_SCRY_LINE.search(line), f"old scry label: {line}")
        if "Does Machines" in text:  # names resolve only with the card DB installed
            self.assertIn("Discard two cards (Does Machines):", text)

    def test_live_tailer_saves_match_when_it_ends(self):
        """Live mode: a match is saved only once its completed event arrives."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            live_log = tmp / "Player.log"
            raw = PLAYER_LOG.read_bytes()
            cut = raw.rfind(b"MatchGameRoomStateType_MatchCompleted")
            self.assertGreater(cut, 0)
            live_log.write_bytes(raw[:cut - 200])

            tailer = PlayerLogTailer(live_log, from_start=True, parser=MatchSlicer())
            finished = tailer.poll()
            self.assertEqual(finished, [], "match must not finish before its end event")

            with open(live_log, "ab") as f:
                f.write(raw[cut - 200:])
            finished = tailer.poll()
            self.assertEqual(len(finished), 1)
            out = save_match(finished[0], tmp / "captures", self.card_db, quiet=True)
            self.assertIsNotNone(out)
            doc = json.loads(out.read_text(encoding="utf-8"))
            self.assertTrue(doc["play_by_play"])

    def test_analyze_writes_decisions_summary_and_report(self):
        out_dir = Path(self._tmp.name) / "analysis"
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            rc = analyze_games.main(["--captures", str(self.captures), "--out", str(out_dir)])
        self.assertEqual(rc, 0)

        decision_files = sorted(out_dir.glob("decisions_*.json"))
        self.assertEqual(len(decision_files), 2, "one decisions file per game")
        games = [json.loads(p.read_text(encoding="utf-8")) for p in decision_files]
        for g in games:
            self.assertIn("won", g["result"])
            self.assertGreaterEqual(g["mulligans"], 0)
            self.assertTrue(g["lands"], "every game plays at least one land")
            self.assertTrue(g["spells"])
            for land in g["lands"]:
                self.assertIn("tapped", land)
                self.assertIn("life_paid", land)

        summary = json.loads((out_dir / "summary.json").read_text(encoding="utf-8"))
        self.assertEqual(summary["games"], 2)
        report = (out_dir / "report.md").read_text(encoding="utf-8")
        self.assertIn("## Forced discards", report)
        self.assertIn("## Decisions that differ from the simulator's engine rules", report)

        # The report and summary never name the players' accounts
        for doc in self.docs:
            for player in doc["players"]:
                self.assertNotIn(player["name"], report)
                self.assertNotIn(player["name"], json.dumps(summary))

        if any("Does Machines" in json.dumps(g) for g in games):
            choices = [c for g in games for c in g["discard_choices"]
                       if c["source"] == "Does Machines"]
            self.assertTrue(choices, "the Does Machines discard choice is extracted")
            self.assertEqual(len(choices[0]["cards"]), 2)

        artifact = {
            "source_logs": [str(PREV_LOG), str(PLAYER_LOG)],
            "captures": [d["match_id"] for d in self.docs],
            "games": summary["games"],
            "wins": summary["wins"],
            "discard_choices": summary["discard_choices"],
            "combo": summary["combo"],
            "rule_differences": summary["rule_differences"],
        }
        artifact_path = Path(tempfile.gettempdir()) / "collect_analyze_report.json"
        artifact_path.write_text(json.dumps(artifact, indent=2), encoding="utf-8")
        print(f"\n[artifact] {artifact_path}\n{json.dumps(artifact, indent=2)}")

    def test_llm_flag_uses_mocked_advisor_client(self):
        """--llm goes through the advisor client; here it is fully mocked."""
        import advisor

        calls = []

        def fake_create(**kwargs):
            calls.append(kwargs)
            return SimpleNamespace(content=[SimpleNamespace(type="text", text="mock discussion")])

        stub = SimpleNamespace(messages=SimpleNamespace(create=fake_create))
        out_dir = Path(self._tmp.name) / "analysis_llm"
        stdout = io.StringIO()
        with mock.patch.object(advisor, "_sync_client", return_value=stub):
            with contextlib.redirect_stdout(stdout):
                rc = analyze_games.main([
                    "--captures", str(self.captures), "--out", str(out_dir), "--llm",
                ])
        self.assertEqual(rc, 0)
        self.assertIn("mock discussion", stdout.getvalue())
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]["model"], advisor.advisor_model())
        prompt = calls[0]["messages"][0]["content"]
        for doc in self.docs:
            for player in doc["players"]:
                self.assertNotIn(player["name"], prompt, "no account names go to the model")


if __name__ == "__main__":
    unittest.main(verbosity=2)
