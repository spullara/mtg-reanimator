# MTGA Live Game Tracker

Tails MTGA's `Player.log`, builds live game state, and shows it on a web
dashboard with an LLM move advisor. No proxy, no root certificate, no sudo,
and no hosts-file changes — Arena writes everything we need to its own log.

## Prerequisites

- Python 3.10+
- `pip3 install -r arena/requirements.txt`
- MTGA with **Detailed Logs** enabled: in Arena, go to
  **Options → Account → Detailed Logs (Plugin Support)** and restart Arena.

## Quick Start

Run every command from the repository root (the folder that contains
`arena/`), so that `python -m arena.collect` can find the package.

macOS:

```bash
pip3 install -r arena/requirements.txt
python3 arena/live_server.py
```

Windows (PowerShell):

```powershell
py -m pip install -r arena\requirements.txt
$env:ANTHROPIC_API_KEY = "sk-ant-..."   # optional: LLM advice
py arena\live_server.py
```

On Windows, use `py` (or `python`) where this README says `python3`.

Then open <http://localhost:8765> and play. The server:

1. Finds `Player.log` and the card database automatically (see below).
2. Tails the log in real time and parses the GRE messages.
3. Pushes game state to the dashboard over WebSocket (port 8766).
4. Asks the advisor for a recommendation at each decision point.
   Set `ANTHROPIC_API_KEY` for LLM advice; without it, a deterministic
   heuristic answers instead. See [Advisor](#advisor).

Options:

| Flag | Meaning |
|------|---------|
| `--log PATH` | Explicit path to `Player.log` |
| `--card-db PATH` | Explicit path to `Raw_CardDatabase_*.mtga` |
| `--from-start` | Replay the existing log content before following |

## File locations

The tools look for the files in these places, in order. Overrides always win.

**Player.log**

| OS | Path |
|----|------|
| Windows | `%USERPROFILE%\AppData\LocalLow\Wizards Of The Coast\MTGA\Player.log` |
| macOS | `~/Library/Logs/Wizards Of The Coast/MTGA/Player.log` |

At each Arena restart, the old log rotates to `Player-prev.log` in the same
folder. The tailer follows the rotation automatically.

**Card database** (`MTGA_Data/Downloads/Raw/Raw_CardDatabase_*.mtga`)

| OS | Install | MTGA_Data location |
|----|---------|--------------------|
| Windows | Wizards | `C:\Program Files\Wizards of the Coast\MTGA\MTGA_Data` |
| Windows | Steam | `C:\Program Files (x86)\Steam\steamapps\common\MTGA\MTGA_Data` |
| Windows | Epic | `C:\Program Files\Epic Games\MagicTheGathering\MTGA_Data` |
| macOS | Steam | `~/Library/Application Support/Steam/steamapps/common/MTGA/MTGA_Data` |
| macOS | Wizards | `/Applications/MTGA.app/Contents/Resources/Data` |

## Advisor

`advisor.py` recommends one of the legal actions that Arena offers at each
decision point. It only advises; the player makes every click.

- **Deck:** read from the match in `Player.log` (the deck the client sends at
  match start), with card names from Arena's card database.
- **Strategy:** a plain-text file in `arena/strategies/`, written in the deck
  designer's own words. The advisor picks the file whose cards match the deck
  best. When the file does not cover a situation, the advisor says so and asks
  instead of guessing. `dimir-reanimator.md` is the first one (a draft).
- **Model:** Claude through the Anthropic Python SDK, with structured outputs
  (a validated JSON answer), prompt caching on the system prompt, and
  server-side refusal fallbacks. On any API error or refusal it falls back to
  the offline heuristic and says why.

| Environment variable | Default | Meaning |
|---|---|---|
| `ANTHROPIC_API_KEY` | (none) | API key; without it the offline heuristic answers |
| `ARENA_ADVISOR_MODEL` | `claude-opus-5-5` | Model ID |
| `ARENA_ADVISOR_EFFORT` | `low` | `low`, `medium`, `high`, `xhigh` or `max` (lower is faster) |
| `ARENA_STRATEGY_FILE` | (auto) | Use this strategy file instead of the best match |
| `ARENA_CAPTURES_DIR` | `arena/captures/` | Where the collector saves matches |

## Terms of Service

These tools read only `Player.log`, the same file that tracker apps such as
Untapped.gg and 17Lands read with Detailed Logs on. They do not modify the
Arena client, do not intercept network traffic, and see no hidden
information. The advisor never acts on its own. Wizards' Terms of Use
(section 2.2) prohibit unauthorized bots and software that gives an advantage
over other players; do not add automated play without written authorization
from Wizards.

## Match archive

Arena overwrites `Player.log` at each start, so at most two sessions
survive on disk. The collector saves every finished match to its own JSON
file: metadata, the parsed play-by-play, and the raw GRE lines, so a match
can be re-parsed later.

```bash
# While Arena runs: save each match when it ends (Ctrl-C to stop)
python3 -m arena.collect

# Archive the matches in Player.log and Player-prev.log now, then exit
python3 -m arena.collect --once
```

Captures go to `arena/captures/` (git-ignored — they hold account and
session data; never commit them). Override with `--dir` or
`ARENA_CAPTURES_DIR`. Matches already saved are skipped by match ID.

## Strategy analysis

```bash
python3 arena/analyze_games.py [--dir DIR] [--out DIR] [--llm]
```

Extracts the player's decisions from the saved captures — mulligans, land
sequencing (tapped / life paid), spells per turn, discard and mill
choices, Beseech the Mirror fetches and bargains, the combo turn and copy
target, interaction, and results. Writes one `decisions_<match>.json` per
game, `summary.json`, and a plain-words `report.md` with the patterns the
games show and the decisions that differ from the simulator's engine
rules. `--llm` also sends the report (never the raw logs, and no account
names) to the advisor's Anthropic model for a written discussion; the
default is fully offline.

## Other tools

```bash
# Play-by-play narrative of the matches in a Player.log
python3 arena/parse_game.py [PLAYER_LOG]

# Advisor replay: advise at each decision point of a saved log (no API calls)
python3 arena/advisor.py --replay PLAYER_LOG --offline
```

## Tests

```bash
python3 -m unittest discover -s arena/tests -v
```

`test_tailer_replay.py` replays a saved log through the tailer in small
chunks (plus truncation and rotation cases) and checks that the events,
the final dashboard state, and the advisor trigger count match the offline
parser. Point `MTGA_SAMPLE_LOG` at a real `Player.log` to run the replay
against it; otherwise a bundled synthetic log is used. The test writes a
JSON report with the replay results and prints its path.

## Files

| File | Description |
|------|-------------|
| `live_server.py` | Log tailer + WebSocket/HTTP server for the dashboard |
| `log_tailer.py` | Incremental Player.log parser and rotation-aware tailer |
| `paths.py` | Cross-platform discovery of Player.log and the card database |
| `mtga_state.py` | Shared live game state built from GRE messages |
| `parse_game.py` | Offline play-by-play parser for saved logs |
| `collect.py` | Match archiver: one capture file per finished match |
| `analyze_games.py` | Decision extraction and strategy report from captures |
| `advisor.py` | LLM move advisor (Anthropic structured outputs) |
| `web/` | Dashboard UI |
| `captures/` | Saved matches (git-ignored; local only) |
