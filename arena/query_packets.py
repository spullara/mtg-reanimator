#!/usr/bin/env python3
"""Query utility for the MTGA packet capture SQLite database."""
import argparse
import sqlite3
import sys
from pathlib import Path

DEFAULT_DB = Path(__file__).parent / "captures" / "mtga_packets.db"


def get_db(path: str) -> sqlite3.Connection:
    if not Path(path).exists():
        print(f"Error: database not found at {path}", file=sys.stderr)
        sys.exit(1)
    conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    return conn


def cmd_summary(db: sqlite3.Connection, _args: argparse.Namespace) -> None:
    print("=== Sessions ===")
    for r in db.execute("SELECT id, started_at, server_host, server_port FROM sessions ORDER BY id"):
        host = r["server_host"] or "unknown"
        port = r["server_port"] or "?"
        print(f"  Session {r['id']}: started {r['started_at']}  server={host}:{port}")

    print("\n=== Packet Counts by Direction ===")
    for r in db.execute("SELECT direction, COUNT(*) as cnt FROM packets GROUP BY direction"):
        print(f"  {r['direction']}: {r['cnt']}")

    print("\n=== JSON vs Binary ===")
    row = db.execute("SELECT COUNT(*) as total, SUM(is_json) as json_cnt FROM packets").fetchone()
    total, json_cnt = row["total"], row["json_cnt"] or 0
    print(f"  Total packets: {total}")
    print(f"  JSON packets:  {json_cnt}")
    print(f"  Binary packets: {total - json_cnt}")

    print("\n=== Total Data ===")
    row = db.execute("SELECT SUM(size) as total_bytes FROM packets").fetchone()
    tb = row["total_bytes"] or 0
    print(f"  {tb:,} bytes ({tb / 1024 / 1024:.2f} MB)")


def cmd_methods(db: sqlite3.Connection, _args: argparse.Namespace) -> None:
    print("=== Method Names ===")
    rows = db.execute(
        "SELECT method_name, COUNT(*) as cnt FROM packets "
        "WHERE method_name IS NOT NULL GROUP BY method_name ORDER BY cnt DESC"
    ).fetchall()
    if not rows:
        print("  No methods found.")
        return
    for r in rows:
        print(f"  {r['cnt']:>5}  {r['method_name']}")
    null_cnt = db.execute("SELECT COUNT(*) as c FROM packets WHERE method_name IS NULL").fetchone()["c"]
    if null_cnt:
        print(f"  {null_cnt:>5}  (no method detected)")


def cmd_search(db: sqlite3.Connection, args: argparse.Namespace) -> None:
    term = args.term
    rows = db.execute(
        "SELECT id, timestamp, direction, method_name, size FROM packets "
        "WHERE json_text LIKE ? ORDER BY timestamp",
        (f"%{term}%",),
    ).fetchall()
    print(f"=== Search: '{term}' ({len(rows)} results) ===")
    for r in rows:
        method = r["method_name"] or "?"
        print(f"  #{r['id']}  {r['timestamp']}  {r['direction']}  {method}  ({r['size']} bytes)")


def cmd_dump(db: sqlite3.Connection, args: argparse.Namespace) -> None:
    row = db.execute("SELECT * FROM packets WHERE id = ?", (args.packet_id,)).fetchone()
    if not row:
        print(f"Packet {args.packet_id} not found.", file=sys.stderr)
        sys.exit(1)
    print(f"Packet #{row['id']}")
    print(f"  Session:   {row['session_id']}")
    print(f"  Timestamp: {row['timestamp']}")
    print(f"  Direction: {row['direction']}")
    print(f"  Client:    {row['client_addr']}")
    print(f"  Size:      {row['size']} bytes")
    print(f"  Is JSON:   {bool(row['is_json'])}")
    print(f"  Method:    {row['method_name']}")
    print(f"  Notes:     {row['notes']}")
    if row["has_binary_header"] and row["binary_header"]:
        print(f"  Header:    {bytes(row['binary_header']).hex(' ')}")
    if row["json_text"]:
        print(f"\n--- JSON Content ---\n{row['json_text']}")
    else:
        raw = bytes(row["raw_data"])
        print(f"\n--- Raw Hex (first 256 bytes) ---\n{raw[:256].hex(' ')}")
        print(f"\n--- Raw Text (first 256 bytes) ---\n{raw[:256].decode('utf-8', errors='replace')}")


def cmd_json(db: sqlite3.Connection, _args: argparse.Namespace) -> None:
    rows = db.execute(
        "SELECT id, timestamp, direction, method_name, size FROM packets "
        "WHERE is_json = 1 ORDER BY timestamp"
    ).fetchall()
    print(f"=== JSON Packets ({len(rows)}) ===")
    for r in rows:
        method = r["method_name"] or "?"
        print(f"  #{r['id']:>5}  {r['timestamp']}  {r['direction']:<20}  {method:<30}  {r['size']:>8} bytes")


def cmd_large(db: sqlite3.Connection, _args: argparse.Namespace) -> None:
    rows = db.execute(
        "SELECT id, timestamp, direction, method_name, size, is_json, notes FROM packets "
        "ORDER BY size DESC LIMIT 25"
    ).fetchall()
    print("=== Largest Packets (top 25) ===")
    for r in rows:
        method = r["method_name"] or "?"
        kind = "JSON" if r["is_json"] else "BIN"
        notes = f"  [{r['notes']}]" if r["notes"] else ""
        print(f"  #{r['id']:>5}  {r['size']:>10,} bytes  {kind}  {r['direction']:<20}  {method}{notes}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Query MTGA packet capture database")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="Path to SQLite database")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("summary", help="Session list, packet counts, JSON vs binary breakdown")
    sub.add_parser("methods", help="List unique method names with counts")

    p_search = sub.add_parser("search", help="Full-text search across json_text")
    p_search.add_argument("term", help="Search term")

    p_dump = sub.add_parser("dump", help="Dump full contents of a packet")
    p_dump.add_argument("packet_id", type=int, help="Packet ID")

    sub.add_parser("json", help="List all JSON packets")
    sub.add_parser("large", help="List largest packets")

    args = parser.parse_args()
    db = get_db(args.db)

    cmds = {
        "summary": cmd_summary, "methods": cmd_methods, "search": cmd_search,
        "dump": cmd_dump, "json": cmd_json, "large": cmd_large,
    }
    cmds[args.command](db, args)


if __name__ == "__main__":
    main()
