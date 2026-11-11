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

```bash
python3 arena/live_server.py
```

Then open <http://localhost:8765> and play. The server:

1. Finds `Player.log` and the card database automatically (see below).
2. Tails the log in real time and parses the GRE messages.
3. Pushes game state to the dashboard over WebSocket (port 8766).
4. Asks the advisor for a recommendation at each decision point.
   Set `ANTHROPIC_API_KEY` for LLM advice; without it, a deterministic
   heuristic answers instead.

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
| `advisor.py` | LLM move advisor (Anthropic structured outputs) |
| `web/` | Dashboard UI |
