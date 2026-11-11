"""
mitmproxy addon script for intercepting MTGA FrontDoor TCP/TLS traffic.

Usage: mitmdump --mode reverse:tcp://REAL_HOST:30010 -p 30010 --tcp-hosts ".*" -s arena/mtga_intercept.py
"""
import json
import os
import re
import sys
import threading
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

from mitmproxy import tcp


CAPTURE_DIR = Path(__file__).parent / "captures"
CAPTURE_DIR.mkdir(exist_ok=True)

# Timestamped capture file for this session
SESSION_START = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
CAPTURE_FILE = CAPTURE_DIR / f"mtga_capture_{SESSION_START}.log"
RAW_FILE = CAPTURE_DIR / f"mtga_raw_{SESSION_START}.bin"

# JSONL packet log for this session (one JSON object per line)
JSONL_FILE = CAPTURE_DIR / f"mtga_packets_{SESSION_START}.jsonl"

# Game state server URL for real-time forwarding
GAME_SERVER_URL = "http://localhost:8765/packet"

# Buffer partial messages per connection+direction
_buffers: dict[str, bytes] = {}


def _forward_to_game_server(direction: str, raw_data: bytes) -> None:
    """POST raw packet data to the game state server (fire-and-forget)."""
    def _post():
        try:
            payload = json.dumps({
                "direction": direction,
                "data": raw_data.hex(),
                "timestamp": datetime.now(timezone.utc).isoformat(),
            }).encode("utf-8")
            req = urllib.request.Request(
                GAME_SERVER_URL,
                data=payload,
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            urllib.request.urlopen(req, timeout=2)
        except Exception:
            pass  # Silently skip if server is down

    t = threading.Thread(target=_post, daemon=True)
    t.start()


def _store_packet_jsonl(
    *,
    direction: str,
    client_addr: str | None,
    raw_data: bytes,
    size: int,
    is_json: bool,
    json_data: object,
    method_name: str | None,
    has_binary_header: bool,
    binary_header: bytes | None,
    notes: str | None,
) -> None:
    """Append a packet record as a single JSON line to the JSONL file."""
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3]
    record = {
        "timestamp": ts,
        "direction": direction,
        "client_addr": client_addr,
        "size": size,
        "is_json": is_json,
        "json_data": json_data,
        "method_name": method_name,
        "has_binary_header": has_binary_header,
        "binary_header_hex": binary_header.hex(" ") if binary_header else None,
        "raw_hex": raw_data.hex(" "),
        "notes": notes,
    }
    with open(JSONL_FILE, "a") as f:
        f.write(json.dumps(record, ensure_ascii=False) + "\n")


def _log(msg: str) -> None:
    """Log to stdout and capture file."""
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
    line = f"[{ts}] {msg}"
    print(line, flush=True)
    with open(CAPTURE_FILE, "a") as f:
        f.write(line + "\n")


def _try_extract_method(data: bytes) -> str | None:
    """Try to extract the method/command name from a FrontDoor message."""
    try:
        text = data.decode("utf-8", errors="replace")
        # Pattern: ==> MethodName {json} or <== MethodName(id) {json}
        m = re.search(r"(?:==>|<==)\s+(\w+)", text)
        if m:
            return m.group(1)
        # Try to find a JSON object with a method/command field
        for match in re.finditer(r"\{[^{}]*\}", text):
            try:
                obj = json.loads(match.group())
                for key in ("method", "Method", "CmdType", "cmdType", "type", "Type", "action", "Action"):
                    if key in obj:
                        return str(obj[key])
            except (json.JSONDecodeError, ValueError):
                continue
    except Exception:
        pass
    return None


def _try_parse_json_objects(data: bytes) -> list[dict]:
    """Try to extract JSON objects from the data, handling length-prefixed framing."""
    results = []
    text = data.decode("utf-8", errors="replace")

    # Try the whole thing as JSON first
    try:
        obj = json.loads(text.strip())
        results.append(obj)
        return results
    except (json.JSONDecodeError, ValueError):
        pass

    # Try to find JSON objects by brace matching
    depth = 0
    start = None
    for i, ch in enumerate(text):
        if ch == "{":
            if depth == 0:
                start = i
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0 and start is not None:
                try:
                    obj = json.loads(text[start : i + 1])
                    results.append(obj)
                except (json.JSONDecodeError, ValueError):
                    pass
                start = None

    return results


def _detect_binary_header(data: bytes) -> tuple[bool, bytes | None]:
    """Check for binary framing header (04 XX ... pattern)."""
    if len(data) >= 2 and data[0] == 0x04:
        # Take 2-6 bytes as framing header
        header_len = min(6, len(data))
        # Find where non-header data starts (after null bytes or at JSON/content boundary)
        for i in range(2, header_len + 1):
            if i < len(data) and (data[i] == 0x7B or data[i] == 0x1F):  # '{' or gzip magic
                return True, data[:i]
        return True, data[:min(6, len(data))]
    return False, None


def _detect_notes(data: bytes) -> str | None:
    """Auto-detect notable features of the packet data."""
    notes_parts = []
    # Check for gzip content
    if b"\x1f\x8b\x08" in data:
        notes_parts.append("gzip-compressed")
    # Check for auth-related content
    text = data.decode("utf-8", errors="replace").lower()
    if "bearer" in text or "authorization" in text or "access_token" in text:
        notes_parts.append("auth token")
    return ", ".join(notes_parts) if notes_parts else None


def tcp_message(flow: tcp.TCPFlow) -> None:
    """Hook called for each TCP message in the stream."""
    msg = flow.messages[-1]
    direction = "==>" if msg.from_client else "<=="
    dir_db = "client_to_server" if msg.from_client else "server_to_client"
    data = msg.content
    size = len(data)

    # Build client address string
    client_addr = None
    if flow.client_conn and flow.client_conn.peername:
        host, port = flow.client_conn.peername[:2]
        client_addr = f"{host}:{port}"

    # Save raw bytes
    with open(RAW_FILE, "ab") as f:
        ts_bytes = datetime.now(timezone.utc).isoformat().encode()
        dir_byte = b"\x01" if msg.from_client else b"\x02"
        f.write(dir_byte + len(ts_bytes).to_bytes(2, "big") + ts_bytes +
                size.to_bytes(4, "big") + data)

    # Forward to game state server (fire-and-forget)
    _forward_to_game_server(dir_db, data)

    # Try to extract method name
    method = _try_extract_method(data)
    method_str = f" {method}" if method else ""

    # Try to parse JSON
    json_objects = _try_parse_json_objects(data)

    # Detect binary header and notes
    has_binary_header, binary_header = _detect_binary_header(data)
    packet_notes = _detect_notes(data)

    # Determine JSON fields
    is_json = bool(json_objects)
    json_data = None
    if json_objects:
        for obj in json_objects:
            # Extract method from object if not found yet
            if not method:
                for key in ("method", "Method", "CmdType", "cmdType", "type", "Type"):
                    if key in obj:
                        method = str(obj[key])
                        method_str = f" {method}"
                        break
                # Also try top-level keys as method name for simple request objects
                if not method:
                    top_keys = [k for k in obj.keys() if k not in (
                        "id", "Id", "payload", "Payload", "params", "Params",
                        "jsonrpc", "result", "error",
                    )]
                    if len(top_keys) == 1:
                        method = top_keys[0]
                        method_str = f" {method}"
            pretty = json.dumps(obj, indent=2, ensure_ascii=False)
            _log(f"{direction}{method_str} ({size} bytes)\n{pretty}")
        # Use list if multiple objects, single object if just one
        json_data = json_objects if len(json_objects) > 1 else json_objects[0]
    else:
        # Log hex preview for non-JSON data
        hex_preview = data[:128].hex(" ")
        text_preview = data[:128].decode("utf-8", errors="replace")
        _log(f"{direction}{method_str} ({size} bytes) [binary/unknown]\n  hex: {hex_preview}\n  txt: {text_preview}")

    # Store in JSONL
    _store_packet_jsonl(
        direction=dir_db,
        client_addr=client_addr,
        raw_data=data,
        size=size,
        is_json=is_json,
        json_data=json_data,
        method_name=method,
        has_binary_header=has_binary_header,
        binary_header=binary_header,
        notes=packet_notes,
    )


_log(f"MTGA FrontDoor interceptor started. Logging to {CAPTURE_FILE}")
_log(f"Raw capture: {RAW_FILE}")
_log(f"JSONL packets: {JSONL_FILE}")
