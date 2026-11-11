#!/usr/bin/env python3
"""Real-time MTGA game state server.

Tails Player.log (requires Detailed Logs in Arena: Options → Account →
Detailed Logs), builds live game state, and pushes updates via WebSocket.

Usage: python3 arena/live_server.py [--log PATH] [--card-db PATH] [--from-start]
"""

import argparse
import asyncio
import copy
import json
import mimetypes
import sys
import traceback
from datetime import datetime, timezone
from http import HTTPStatus
from pathlib import Path

try:
    from websockets.asyncio.server import serve as ws_serve
except ImportError:
    print("ERROR: websockets package required. Install with: pip3 install websockets")
    sys.exit(1)

sys.path.insert(0, str(Path(__file__).parent))
from advisor import get_advice_async, summarize_deck_composition
from log_tailer import PlayerLogTailer
from mtga_state import CardDatabase, GameState
from paths import find_player_log

HOST = "localhost"
HTTP_PORT = 8765
WS_PORT = 8766
WEB_DIR = Path(__file__).parent / "web"
POLL_INTERVAL = 0.25

# Global state
game_state = GameState()
ws_clients: set = set()
_advisor_decision_id = 0
_advisor_task = None
_last_advisor_message = None


def has_actions_available_req(parsed):
    """Return True when a parsed payload contains an action decision prompt."""
    gre = parsed.get("greToClientEvent", {}) if isinstance(parsed, dict) else {}
    for msg in gre.get("greToClientMessages", []):
        if msg.get("type") == "GREMessageType_ActionsAvailableReq":
            return True
    return False


def apply_events(events):
    """Feed tailer events to the game state. Returns (changed, advisor_requested)."""
    changed = False
    advisor_requested = False
    for _event_type, data in events:
        try:
            if game_state.update(data):
                changed = True
                advisor_requested = advisor_requested or has_actions_available_req(data)
                ts = datetime.now(timezone.utc).strftime("%H:%M:%S")
                print(f"[{ts}] State updated from Player.log", flush=True)
        except Exception as e:
            print(f"[ERROR] State update failed: {e}", flush=True)
            traceback.print_exc()
    return changed, advisor_requested


def _action_index(action, fallback):
    if isinstance(action, dict):
        value = action.get("actionIndex", fallback)
    else:
        value = fallback
    try:
        return int(value)
    except (TypeError, ValueError):
        return fallback


def _action_name(action):
    if not isinstance(action, dict):
        return str(action)
    return str(action.get("name") or action.get("actionType") or action.get("actionIndex") or "Action")


def enrich_recommendation(recommendation, actions):
    """Attach display names to advisor action indexes for the browser."""
    action_lookup = {_action_index(action, i): action for i, action in enumerate(actions)}
    result = dict(recommendation or {})
    rec_idx = result.get("recommended_action")
    try:
        rec_idx = int(rec_idx)
    except (TypeError, ValueError):
        rec_idx = None
    if rec_idx in action_lookup:
        result["recommended_action_name"] = _action_name(action_lookup[rec_idx])
    elif rec_idx is not None:
        result["recommended_action_name"] = f"Action {rec_idx}"

    alternatives = []
    for alt in result.get("alternatives", [])[:3]:
        if not isinstance(alt, dict):
            continue
        item = dict(alt)
        alt_idx = item.get("action")
        try:
            alt_idx = int(alt_idx)
        except (TypeError, ValueError):
            alt_idx = None
        if alt_idx in action_lookup:
            item["name"] = _action_name(action_lookup[alt_idx])
        elif alt_idx is not None:
            item["name"] = f"Action {alt_idx}"
        alternatives.append(item)
    result["alternatives"] = alternatives
    return result


async def broadcast_json(message):
    """Push a JSON message to all connected WebSocket clients."""
    global _last_advisor_message
    if isinstance(message, dict) and message.get("type") == "advisor":
        _last_advisor_message = message
    if not ws_clients:
        return
    message_json = json.dumps(message)
    dead = set()
    for ws in ws_clients:
        try:
            await ws.send(message_json)
        except Exception:
            dead.add(ws)
    ws_clients -= dead


async def run_advisor(decision_id, state_snapshot, actions):
    """Compute an advisor recommendation without blocking state updates."""
    await broadcast_json({"type": "advisor", "status": "thinking", "decisionId": decision_id})
    try:
        deck_info = summarize_deck_composition(state_snapshot)
        recommendation = await get_advice_async(
            state_snapshot,
            actions,
            state_snapshot.get("log", []),
            deck_info,
        )
        await broadcast_json({
            "type": "advisor",
            "status": "ready",
            "decisionId": decision_id,
            "recommendation": enrich_recommendation(recommendation, actions),
        })
        ts = datetime.now(timezone.utc).strftime("%H:%M:%S")
        print(f"[{ts}] Advisor recommendation ready", flush=True)
    except asyncio.CancelledError:
        raise
    except Exception as e:
        print(f"[ERROR] Advisor failed: {e}", flush=True)
        traceback.print_exc()
        await broadcast_json({
            "type": "advisor",
            "status": "error",
            "decisionId": decision_id,
            "error": str(e),
        })


def request_advisor_recommendation():
    """Schedule advice for the current decision point."""
    global _advisor_decision_id, _advisor_task
    state_snapshot = copy.deepcopy(game_state.to_dict())
    actions = state_snapshot.get("actionsAvailable", [])
    if not actions:
        return
    _advisor_decision_id += 1
    if _advisor_task and not _advisor_task.done():
        _advisor_task.cancel()
    _advisor_task = asyncio.create_task(run_advisor(_advisor_decision_id, state_snapshot, actions))


async def broadcast_state():
    """Push current game state to all connected WebSocket clients."""
    await broadcast_json(game_state.to_dict())


async def tail_log(tailer):
    """Poll Player.log for new events and publish state changes."""
    while True:
        events = await asyncio.to_thread(tailer.poll)
        if events:
            changed, advisor_requested = apply_events(events)
            if changed:
                await broadcast_state()
            if advisor_requested:
                request_advisor_recommendation()
        else:
            await asyncio.sleep(POLL_INTERVAL)


async def handle_ws(websocket):
    """Handle a WebSocket connection."""
    ws_clients.add(websocket)
    addr = websocket.remote_address
    print(f"[WS] Client connected: {addr}", flush=True)
    try:
        # Send current state immediately
        await websocket.send(json.dumps(game_state.to_dict()))
        if _last_advisor_message:
            await websocket.send(json.dumps(_last_advisor_message))
        async for _ in websocket:
            pass  # We don't expect messages from the client
    except Exception:
        pass
    finally:
        ws_clients.discard(websocket)
        print(f"[WS] Client disconnected: {addr}", flush=True)


class HTTPHandler:
    """Simple async HTTP handler using asyncio streams."""

    def __init__(self, reader, writer):
        self.reader = reader
        self.writer = writer

    async def handle(self):
        try:
            request_line = await asyncio.wait_for(self.reader.readline(), timeout=10)
            if not request_line:
                return
            request_text = request_line.decode('utf-8', errors='replace').strip()
            parts = request_text.split(' ')
            if len(parts) < 2:
                return
            method, path = parts[0], parts[1]

            # Read and discard headers
            while True:
                hline = await asyncio.wait_for(self.reader.readline(), timeout=5)
                if hline in (b'\r\n', b'\n', b''):
                    break

            if method == 'GET' and path == '/state':
                await self._send_json(game_state.to_dict())
            elif method == 'GET':
                await self._serve_static(path)
            else:
                await self._send_response(404, b'Not Found')
        except Exception as e:
            try:
                await self._send_response(500, str(e).encode())
            except Exception:
                pass
        finally:
            try:
                self.writer.close()
                await self.writer.wait_closed()
            except Exception:
                pass

    async def _serve_static(self, path):
        if path == '/' or path == '':
            path = '/index.html'
        file_path = WEB_DIR / path.lstrip('/')
        try:
            file_path = file_path.resolve()
            if not str(file_path).startswith(str(WEB_DIR.resolve())):
                await self._send_response(403, b'Forbidden')
                return
            if not file_path.exists() or not file_path.is_file():
                await self._send_response(404, b'Not Found')
                return
            content = file_path.read_bytes()
            mime, _ = mimetypes.guess_type(str(file_path))
            mime = mime or 'application/octet-stream'
            header = (
                f"HTTP/1.1 200 OK\r\n"
                f"Content-Type: {mime}\r\n"
                f"Content-Length: {len(content)}\r\n"
                f"Connection: close\r\n"
                f"\r\n"
            )
            self.writer.write(header.encode() + content)
            await self.writer.drain()
        except Exception:
            await self._send_response(404, b'Not Found')

    async def _send_json(self, data):
        body = json.dumps(data, indent=2).encode()
        header = (
            f"HTTP/1.1 200 OK\r\n"
            f"Content-Type: application/json\r\n"
            f"Content-Length: {len(body)}\r\n"
            f"Connection: close\r\n"
            f"\r\n"
        )
        self.writer.write(header.encode() + body)
        await self.writer.drain()

    async def _send_response(self, code, body):
        status = HTTPStatus(code).phrase
        header = (
            f"HTTP/1.1 {code} {status}\r\n"
            f"Content-Type: text/plain\r\n"
            f"Content-Length: {len(body)}\r\n"
            f"Connection: close\r\n"
            f"\r\n"
        )
        self.writer.write(header.encode() + body)
        await self.writer.drain()


async def handle_http_connection(reader, writer):
    handler = HTTPHandler(reader, writer)
    await handler.handle()


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description="Real-time MTGA game state server")
    parser.add_argument("--log", metavar="PATH", help="Path to Player.log (default: auto-detect)")
    parser.add_argument("--card-db", metavar="PATH", help="Path to Raw_CardDatabase_*.mtga (default: auto-detect)")
    parser.add_argument("--from-start", action="store_true",
                        help="replay the existing log content before following (default: follow new content only)")
    return parser.parse_args(argv)


async def main(args):
    global game_state

    log_path = find_player_log(args.log)
    if not log_path:
        print("ERROR: Player.log not found. Pass --log PATH, and enable", file=sys.stderr)
        print("Detailed Logs in Arena (Options → Account → Detailed Logs).", file=sys.stderr)
        sys.exit(1)

    game_state = GameState(CardDatabase(args.card_db))
    tailer = PlayerLogTailer(log_path, from_start=args.from_start)

    # Start HTTP server
    http_server = await asyncio.start_server(handle_http_connection, HOST, HTTP_PORT)
    print(f"HTTP server listening on http://{HOST}:{HTTP_PORT}", flush=True)
    print(f"  GET /         — serve web UI from {WEB_DIR}", flush=True)
    print(f"  GET /state    — current game state JSON", flush=True)

    # Start WebSocket server
    await ws_serve(handle_ws, HOST, WS_PORT)
    print(f"WebSocket server listening on ws://{HOST}:{WS_PORT}", flush=True)

    print(f"\nTailing {log_path}", flush=True)

    await asyncio.gather(
        http_server.serve_forever(),
        tail_log(tailer),
    )


if __name__ == "__main__":
    try:
        asyncio.run(main(parse_args()))
    except KeyboardInterrupt:
        print("\nShutting down.", flush=True)
