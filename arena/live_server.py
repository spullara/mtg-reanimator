#!/usr/bin/env python3
"""Real-time MTGA game state server.

Receives raw packets from mitmproxy addon via HTTP POST, parses the FrontDoor
binary protocol, builds live game state, and pushes updates via WebSocket.

Usage: python3 arena/live_server.py
"""

import asyncio
import copy
import gzip
import json
import mimetypes
import struct
import sys
import traceback
from datetime import datetime, timezone
from http import HTTPStatus
from pathlib import Path

try:
    import websockets
    from websockets.asyncio.server import serve as ws_serve
except ImportError:
    print("ERROR: websockets package required. Install with: pip3 install websockets")
    sys.exit(1)

sys.path.insert(0, str(Path(__file__).parent))
from advisor import get_advice_async, summarize_deck_composition
from mtga_state import GameState, CardDatabase

HOST = "localhost"
HTTP_PORT = 8765
WS_PORT = 8766
WEB_DIR = Path(__file__).parent / "web"

# Global state
game_state = GameState()
ws_clients: set = set()
_advisor_decision_id = 0
_advisor_task = None
_last_advisor_message = None
# Buffers for TCP stream reassembly per direction
_buffers: dict[str, bytearray] = {
    "client_to_server": bytearray(),
    "server_to_client": bytearray(),
}


def read_varint(data, pos):
    result, shift = 0, 0
    while pos < len(data):
        b = data[pos]
        result |= (b & 0x7f) << shift
        pos += 1
        if not (b & 0x80):
            break
        shift += 7
    return result, pos


def parse_payload(payload):
    """Parse a FrontDoor data frame payload into a JSON dict (or None)."""
    try:
        start = 0
        # Skip optional UUID header: 0a 24 <36 bytes>
        if len(payload) > 2 and payload[0:1] == b'\x0a' and payload[1:2] == b'\x24':
            start = 38
        inner = payload[start:]
        if not inner:
            return None

        # Read protobuf field 3 tag + length
        tag, p = read_varint(inner, 0)
        length, p = read_varint(inner, p)
        field_data = inner[p:p + length]

        # Look for gzip within the field data
        gzip_idx = field_data.find(b'\x1f\x8b\x08')
        if gzip_idx >= 0:
            decompressed = gzip.decompress(field_data[gzip_idx:])
            return json.loads(decompressed)

        # Try direct JSON
        json_start = inner.find(b'{')
        if json_start >= 0:
            return json.loads(inner[json_start:])
    except Exception:
        pass
    return None


def has_actions_available_req(parsed):
    """Return True when a parsed payload contains an action decision prompt."""
    gre = parsed.get("greToClientEvent", {}) if isinstance(parsed, dict) else {}
    for msg in gre.get("greToClientMessages", []):
        if msg.get("type") == "GREMessageType_ActionsAvailableReq":
            return True
    return False


def process_frames(direction):
    """Extract complete frames from the buffer, parse them, update state.

    Frame format: 04 XX <4-byte LE length> <payload>
    Returns (changed, advisor_requested).
    """
    buf = _buffers[direction]
    changed = False
    advisor_requested = False

    while len(buf) >= 6:
        if buf[0] != 0x04:
            # Not a valid frame start — skip one byte
            del buf[0]
            continue

        frame_type = buf[1]
        payload_len = struct.unpack_from('<I', buf, 2)[0]

        total_frame = 6 + payload_len
        if len(buf) < total_frame:
            break  # Wait for more data

        payload = bytes(buf[6:total_frame])
        del buf[:total_frame]

        # Only parse data frames (0x21)
        if frame_type != 0x21:
            continue

        parsed = parse_payload(payload)
        if parsed is None:
            continue

        # Feed to game state
        try:
            if game_state.update(parsed):
                changed = True
                advisor_requested = advisor_requested or has_actions_available_req(parsed)
                ts = datetime.now(timezone.utc).strftime("%H:%M:%S")
                print(f"[{ts}] State updated from {direction}", flush=True)
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

            # Read headers
            headers = {}
            content_length = 0
            while True:
                hline = await asyncio.wait_for(self.reader.readline(), timeout=5)
                if hline in (b'\r\n', b'\n', b''):
                    break
                htext = hline.decode('utf-8', errors='replace').strip()
                if ':' in htext:
                    k, v = htext.split(':', 1)
                    headers[k.strip().lower()] = v.strip()
            content_length = int(headers.get('content-length', 0))

            if method == 'POST' and path == '/packet':
                await self._handle_packet(content_length)
            elif method == 'GET' and path == '/state':
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

    async def _handle_packet(self, content_length):
        if content_length == 0:
            await self._send_response(400, b'No body')
            return

        body = await asyncio.wait_for(self.reader.readexactly(content_length), timeout=10)
        try:
            packet = json.loads(body)
        except json.JSONDecodeError:
            await self._send_response(400, b'Invalid JSON')
            return

        direction = packet.get("direction", "server_to_client")
        hex_data = packet.get("data", "")
        try:
            raw_bytes = bytes.fromhex(hex_data)
        except ValueError:
            await self._send_response(400, b'Invalid hex data')
            return

        _buffers[direction].extend(raw_bytes)
        changed, advisor_requested = process_frames(direction)

        await self._send_response(200, b'OK')

        if changed:
            await broadcast_state()
        if advisor_requested:
            request_advisor_recommendation()

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


async def main():
    # Start HTTP server
    http_server = await asyncio.start_server(handle_http_connection, HOST, HTTP_PORT)
    print(f"HTTP server listening on http://{HOST}:{HTTP_PORT}", flush=True)
    print(f"  POST /packet  — receive raw packets from mitmproxy", flush=True)
    print(f"  GET /         — serve web UI from {WEB_DIR}", flush=True)
    print(f"  GET /state    — current game state JSON", flush=True)

    # Start WebSocket server
    ws_server = await ws_serve(handle_ws, HOST, WS_PORT)
    print(f"WebSocket server listening on ws://{HOST}:{WS_PORT}", flush=True)

    print("\nWaiting for packets...", flush=True)

    # Run both servers
    await asyncio.gather(
        http_server.serve_forever(),
        asyncio.Future(),  # run forever
    )


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nShutting down.", flush=True)
