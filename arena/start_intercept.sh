#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_PORT=30010
MITMDUMP="${MITMDUMP:-/opt/homebrew/bin/mitmdump}"
PLAYER_LOG="$HOME/Library/Logs/Wizards Of The Coast/MTGA/Player.log"

# Parse --host argument if provided
FD_HOST=""
FD_PORT=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --host)
            FD_HOST="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

echo "=== MTGA FrontDoor Interceptor ==="
echo ""

# Resolve FrontDoor hostname
echo "[1/3] Resolving FrontDoor hostname..."

if [ -z "$FD_HOST" ]; then
    # Primary: parse from MTGA Player.log
    if [ -f "$PLAYER_LOG" ]; then
        echo "  Parsing Player.log..."
        FD_MATCH=$(grep -oE 'tcp://frontdoor-[^:"]+' "$PLAYER_LOG" | tail -1 | sed 's|tcp://||' || true)
        if [ -n "$FD_MATCH" ]; then
            FD_HOST="$FD_MATCH"
            echo "  Found in Player.log: $FD_HOST"
        fi
    fi
fi

if [ -z "$FD_HOST" ]; then
    echo "ERROR: Could not determine FrontDoor hostname."
    echo ""
    echo "Options:"
    echo "  1. Launch MTGA once so Player.log contains the doorbell response, then re-run"
    echo "  2. Pass --host manually:"
    echo "     $0 --host frontdoor-mtga-production-YYYY-XX-XX-X.w2.mtgarena.com"
    exit 1
fi

# Extract port from Player.log if we got a full URI, otherwise default to 30010
if [ -z "$FD_PORT" ]; then
    FD_PORT_MATCH=$(grep -oE "tcp://${FD_HOST}:[0-9]+" "$PLAYER_LOG" 2>/dev/null | tail -1 | sed 's|.*:||' || true)
    if [ -n "$FD_PORT_MATCH" ]; then
        FD_PORT="$FD_PORT_MATCH"
    else
        FD_PORT="30010"
    fi
fi

echo "  FrontDoor server: $FD_HOST:$FD_PORT"
echo ""

# Check if hosts redirect is already active (would poison DNS resolution)
if grep -q "MTGA-MITMPROXY-INTERCEPT" /etc/hosts 2>/dev/null; then
    echo "⚠️  WARNING: /etc/hosts redirect is already active!"
    echo "   DNS resolution via socket will return 127.0.0.1 (loop-back)."
    echo "   Recommended order:"
    echo "     1. sudo bash $SCRIPT_DIR/setup_hosts.sh --restore"
    echo "     2. bash $SCRIPT_DIR/start_intercept.sh"
    echo "     3. sudo bash $SCRIPT_DIR/setup_hosts.sh --add"
    echo ""
fi

# Resolve real IP address (bypasses /etc/hosts to avoid loop-back)
echo "  Resolving real IP for $FD_HOST..."
FD_IP=""
if command -v dig &>/dev/null; then
    FD_IP=$(dig +short "$FD_HOST" | grep -E '^[0-9]+\.' | head -1 || true)
fi
if [ -z "$FD_IP" ]; then
    echo "  dig not available or returned no result, falling back to Python..."
    if grep -q "MTGA-MITMPROXY-INTERCEPT" /etc/hosts 2>/dev/null; then
        echo "  ⚠️  WARNING: /etc/hosts redirect is active and fallback uses system resolver."
        echo "     The resolved IP may be 127.0.0.1 (loop-back). Consider installing dig:"
        echo "       brew install bind"
    fi
    FD_IP=$(python3 -c "import socket; print(socket.getaddrinfo('$FD_HOST', None, socket.AF_INET)[0][4][0])" 2>/dev/null || true)
fi
if [ -z "$FD_IP" ]; then
    echo "  ERROR: Could not resolve IP for $FD_HOST"
    exit 1
fi
echo "  Resolved IP: $FD_IP"
echo ""

# Verify mitmdump is available
if [ ! -x "$MITMDUMP" ]; then
    echo "ERROR: mitmdump not found at $MITMDUMP"
    echo "Install with: brew install mitmproxy"
    exit 1
fi

echo "[2/3] Pre-flight checks..."
echo "  mitmdump: $($MITMDUMP --version 2>&1 | head -1)"
echo ""

echo "[3/3] Starting mitmdump in reverse TCP mode..."
echo ""
echo "  Listening on:   localhost:$LOCAL_PORT"
echo "  Forwarding to:  $FD_IP:$FD_PORT ($FD_HOST)"
echo "  Addon script:   $SCRIPT_DIR/mtga_intercept.py"
echo "  Captures dir:   $SCRIPT_DIR/captures/"
echo ""
echo "============================================"
echo "  BEFORE LAUNCHING MTGA, make sure you:"
echo ""
echo "  1. Trust the mitmproxy CA cert (one-time):"
echo "     sudo security add-trusted-cert -d -r trustRoot \\"
echo "       -k /Library/Keychains/System.keychain \\"
echo "       ~/.mitmproxy/mitmproxy-ca-cert.pem"
echo ""
echo "  2. Redirect the FrontDoor hostname to localhost:"
echo "     sudo bash $SCRIPT_DIR/setup_hosts.sh --add"
echo ""
echo "  3. Launch MTGA and play normally"
echo ""
echo "  To stop: press Ctrl+C, then restore hosts:"
echo "     sudo bash $SCRIPT_DIR/setup_hosts.sh --restore"
echo "============================================"
echo ""

exec "$MITMDUMP" \
    --mode "reverse:tls://$FD_IP:$FD_PORT" \
    -p "$LOCAL_PORT" \
    --tcp-hosts ".*" \
    --ssl-insecure \
    -s "$SCRIPT_DIR/mtga_intercept.py"
