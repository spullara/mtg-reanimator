#!/usr/bin/env python3
"""Import MTGA packet JSONL captures into SQLite for querying.

Usage:
    python3 arena/import_packets.py [file1.jsonl] [file2.jsonl ...]
    python3 arena/import_packets.py  # imports all .jsonl in arena/captures/
"""
import json
import sqlite3
import sys
from pathlib import Path

CAPTURE_DIR = Path(__file__).parent / "captures"
DB_PATH = CAPTURE_DIR / "mtga_packets.db"


def init_db(conn: sqlite3.Connection) -> None:
    """Create tables if they don't exist."""
    conn.execute("PRAGMA journal_mode=WAL")
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            started_at TEXT NOT NULL,
            server_host TEXT,
            server_port INTEGER
        );
        CREATE TABLE IF NOT EXISTS packets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL REFERENCES sessions(id),
            timestamp TEXT NOT NULL,
            direction TEXT NOT NULL,
            client_addr TEXT,
            raw_data BLOB NOT NULL,
            size INTEGER NOT NULL,
            is_json BOOLEAN DEFAULT FALSE,
            json_text TEXT,
            method_name TEXT,
            has_binary_header BOOLEAN DEFAULT FALSE,
            binary_header BLOB,
            notes TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_packets_session ON packets(session_id);
        CREATE INDEX IF NOT EXISTS idx_packets_direction ON packets(direction);
        CREATE INDEX IF NOT EXISTS idx_packets_method ON packets(method_name);
        CREATE INDEX IF NOT EXISTS idx_packets_timestamp ON packets(timestamp);
    """)


def import_jsonl(conn: sqlite3.Connection, jsonl_path: Path) -> int:
    """Import a single JSONL file. Returns number of packets imported."""
    # Extract session start time from filename: mtga_packets_YYYYMMDD_HHMMSS.jsonl
    stem = jsonl_path.stem  # e.g. "mtga_packets_20260407_222416"
    parts = stem.split("_")
    if len(parts) >= 4:
        started_at = f"{parts[-2][:4]}-{parts[-2][4:6]}-{parts[-2][6:8]}T{parts[-1][:2]}:{parts[-1][2:4]}:{parts[-1][4:6]}Z"
    else:
        started_at = stem

    cur = conn.execute("INSERT INTO sessions (started_at) VALUES (?)", (started_at,))
    session_id = cur.lastrowid

    # Build set of existing (timestamp, direction, size) for dedup
    existing = set(
        conn.execute(
            "SELECT timestamp, direction, size FROM packets"
        ).fetchall()
    )

    imported = 0
    with open(jsonl_path, "r") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            record = json.loads(line)

            ts = record["timestamp"]
            direction = record["direction"]
            size = record["size"]

            # Deduplicate
            if (ts, direction, size) in existing:
                continue

            # Reconstruct raw_data from hex
            raw_hex = record.get("raw_hex", "")
            raw_data = bytes.fromhex(raw_hex.replace(" ", "")) if raw_hex else b""

            # Reconstruct json_text from json_data
            json_data = record.get("json_data")
            is_json = record.get("is_json", False)
            json_text = None
            if json_data is not None:
                if isinstance(json_data, list):
                    json_text = "\n".join(
                        json.dumps(obj, indent=2, ensure_ascii=False) for obj in json_data
                    )
                else:
                    json_text = json.dumps(json_data, indent=2, ensure_ascii=False)

            # Reconstruct binary_header from hex
            header_hex = record.get("binary_header_hex")
            binary_header = bytes.fromhex(header_hex.replace(" ", "")) if header_hex else None

            conn.execute(
                """INSERT INTO packets
                   (session_id, timestamp, direction, client_addr, raw_data, size,
                    is_json, json_text, method_name, has_binary_header, binary_header, notes)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (session_id, ts, direction, record.get("client_addr"),
                 raw_data, size, is_json, json_text, record.get("method_name"),
                 record.get("has_binary_header", False), binary_header,
                 record.get("notes")),
            )
            existing.add((ts, direction, size))
            imported += 1

    conn.commit()
    return imported


def main() -> None:
    files = [Path(a) for a in sys.argv[1:]] if len(sys.argv) > 1 else sorted(CAPTURE_DIR.glob("*.jsonl"))

    if not files:
        print("No JSONL files found.", file=sys.stderr)
        sys.exit(1)

    CAPTURE_DIR.mkdir(exist_ok=True)
    conn = sqlite3.connect(str(DB_PATH))
    init_db(conn)

    total = 0
    for f in files:
        if not f.exists():
            print(f"Warning: {f} not found, skipping.", file=sys.stderr)
            continue
        count = import_jsonl(conn, f)
        total += count
        print(f"Imported {count} packets from {f.name}")

    conn.close()
    print(f"\nTotal: {total} packets imported into {DB_PATH}")


if __name__ == "__main__":
    main()
