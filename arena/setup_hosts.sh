#!/usr/bin/env bash
set -euo pipefail

HOSTS_FILE="/etc/hosts"
MARKER="# MTGA-MITMPROXY-INTERCEPT"
PLAYER_LOG="$HOME/Library/Logs/Wizards Of The Coast/MTGA/Player.log"

usage() {
    echo "Usage: sudo $0 [--add | --restore] [--host HOSTNAME]"
    echo ""
    echo "  --add      Redirect FrontDoor hostname to 127.0.0.1"
    echo "  --restore  Remove the redirect entry from /etc/hosts"
    echo "  --host     Specify FrontDoor hostname (optional, auto-detected from Player.log)"
    echo ""
    echo "Requires sudo for /etc/hosts modification."
    exit 1
}

get_frontdoor_host() {
    # Primary: parse from MTGA Player.log
    if [ -f "$PLAYER_LOG" ]; then
        local match
        match=$(grep -oE 'tcp://frontdoor-[^:"]+' "$PLAYER_LOG" | tail -1 | sed 's|tcp://||' || true)
        if [ -n "$match" ]; then
            echo "$match"
            return 0
        fi
    fi
    echo ""
    return 1
}

do_add() {
    # First remove any existing entry
    do_restore_quiet

    local host="$OVERRIDE_HOST"
    if [ -z "$host" ]; then
        echo "[*] Resolving FrontDoor hostname from Player.log..."
        host=$(get_frontdoor_host) || true
        if [ -z "$host" ]; then
            echo "ERROR: Could not determine FrontDoor hostname."
            echo "  Options:"
            echo "    1. Launch MTGA once so Player.log has the doorbell response, then re-run"
            echo "    2. Pass --host manually:"
            echo "       sudo $0 --add --host frontdoor-mtga-production-YYYY-XX-XX-X.w2.mtgarena.com"
            exit 1
        fi
    fi
    echo "    FrontDoor host: $host"

    echo "[*] Adding /etc/hosts entry: 127.0.0.1 $host"

    # Create backup
    cp "$HOSTS_FILE" "${HOSTS_FILE}.mtga-backup"

    # Append the entry
    echo "127.0.0.1    $host    $MARKER" >> "$HOSTS_FILE"

    echo "[✓] Done. MTGA will now connect to localhost:30010"
    echo "    Backup saved to ${HOSTS_FILE}.mtga-backup"

    # Flush DNS cache so the redirect takes effect immediately
    echo "[*] Flushing DNS cache..."
    dscacheutil -flushcache 2>/dev/null || true
    killall -HUP mDNSResponder 2>/dev/null || true
    echo "[✓] DNS cache flushed."
    echo ""
    echo "    To verify: ping -c1 $host"
}

do_restore_quiet() {
    # Remove marker lines silently
    if grep -q "$MARKER" "$HOSTS_FILE" 2>/dev/null; then
        local tmp
        tmp=$(mktemp)
        grep -v "$MARKER" "$HOSTS_FILE" > "$tmp"
        mv "$tmp" "$HOSTS_FILE"
        chmod 644 "$HOSTS_FILE"
    fi
}

do_restore() {
    echo "[*] Removing MTGA intercept entries from /etc/hosts..."
    if grep -q "$MARKER" "$HOSTS_FILE"; then
        do_restore_quiet
        echo "[✓] Done. /etc/hosts restored."
    else
        echo "[*] No MTGA intercept entries found in /etc/hosts."
    fi
    # Flush DNS cache
    echo "[*] Flushing DNS cache..."
    dscacheutil -flushcache 2>/dev/null || true
    killall -HUP mDNSResponder 2>/dev/null || true
    echo "[✓] DNS cache flushed."
}

# --- Main ---

OVERRIDE_HOST=""
ACTION=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --add)
            ACTION="add"
            shift
            ;;
        --restore)
            ACTION="restore"
            shift
            ;;
        --host)
            OVERRIDE_HOST="$2"
            shift 2
            ;;
        *)
            usage
            ;;
    esac
done

if [ -z "$ACTION" ]; then
    usage
fi

# Check for root
if [ "$(id -u)" -ne 0 ]; then
    echo "ERROR: This script requires sudo."
    echo "  sudo $0 --$ACTION"
    exit 1
fi

case "$ACTION" in
    add)
        do_add
        ;;
    restore)
        do_restore
        ;;
esac
