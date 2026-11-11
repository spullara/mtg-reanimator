# MTGA FrontDoor Traffic Interceptor

Intercepts MTGA's FrontDoor TCP/TLS connection using mitmproxy in reverse proxy mode to capture and log all API traffic.

## Prerequisites

- macOS with Homebrew
- mitmproxy 12.x (`brew install mitmproxy`)
- Python 3.10+

## Quick Start

### 1. Trust the mitmproxy CA certificate (one-time)

Run mitmdump once to generate the CA cert, then trust it:

```bash
# Generate the CA cert (starts and immediately stop mitmdump)
mitmdump --mode regular -p 9999 &
sleep 2 && kill %1

# Trust the CA cert in macOS system keychain
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain \
  ~/.mitmproxy/mitmproxy-ca-cert.pem
```

### 2. Start the interceptor (resolves real server IP)

```bash
bash arena/start_intercept.sh
```

This will:
- Parse the FrontDoor hostname from MTGA's Player.log
- Resolve the real server IP via DNS (before any /etc/hosts redirect)
- Start mitmdump in reverse TCP mode on port 30010, forwarding to the real IP
- Load the addon script that logs all traffic

> **Important:** Start the interceptor *before* adding the hosts redirect. The script resolves the real server IP using DNS — if the hosts redirect is already active, `dig` will still work (it bypasses /etc/hosts), but the Python fallback would resolve to 127.0.0.1 causing a loop-back.

### 3. Redirect MTGA to localhost

In a separate terminal:

```bash
sudo bash arena/setup_hosts.sh --add
```

This gets the FrontDoor hostname, adds a `/etc/hosts` entry pointing it to `127.0.0.1`, and flushes the DNS cache.

### 4. Launch MTGA and play

MTGA will connect to localhost:30010, which mitmproxy forwards to the real server. All traffic is logged to:
- **stdout** — live view of API calls
- **arena/captures/mtga_capture_YYYYMMDD_HHMMSS.log** — timestamped text log
- **arena/captures/mtga_raw_YYYYMMDD_HHMMSS.bin** — raw binary capture

### 5. Stop and clean up

1. Press `Ctrl+C` in the mitmdump terminal
2. Restore `/etc/hosts`:
   ```bash
   sudo bash arena/setup_hosts.sh --restore
   ```

## How It Works

```
MTGA Client
    │
    │ TLS connect to frontdoor-*.w2.mtgarena.com:30010
    │ (redirected to 127.0.0.1 via /etc/hosts)
    ▼
mitmproxy (localhost:30010)
    │ reverse:tcp mode
    │ terminates TLS with mitmproxy CA cert
    │ re-encrypts to real server
    ▼
Real FrontDoor Server (:30010)
```

## Log Format

The addon logs messages in this format:
```
[2026-04-07 21:30:15.123] ==> MethodName (1234 bytes)
{
  "pretty": "printed json"
}

[2026-04-07 21:30:15.456] <== MethodName (5678 bytes)
{
  "response": "data"
}
```

- `==>` = client → server (request)
- `<==` = server → client (response)

## Troubleshooting

**MTGA won't connect:**
- Verify the hosts redirect: `ping frontdoor-mtga-production-XXX.w2.mtgarena.com` should resolve to `127.0.0.1`
- Make sure mitmdump is running on port 30010
- Check that the CA cert is trusted: Keychain Access → System → look for "mitmproxy"

**TLS errors:**
- Re-run the CA cert trust command
- Make sure you're using the cert from `~/.mitmproxy/mitmproxy-ca-cert.pem`

**No traffic captured:**
- MTGA may use a different FrontDoor server. Re-run `start_intercept.sh` to get the latest hostname
- Check if MTGA is actually running and connected

## Removing mitmproxy CA Trust

To untrust the mitmproxy CA certificate:

```bash
sudo security remove-trusted-cert -d ~/.mitmproxy/mitmproxy-ca-cert.pem
```

Or manually remove it from Keychain Access → System → find "mitmproxy" → Delete.

## Files

| File | Description |
|------|-------------|
| `mtga_intercept.py` | mitmproxy addon script — TCP message logging and JSON parsing |
| `start_intercept.sh` | Queries doorbell, starts mitmdump with correct target |
| `setup_hosts.sh` | Manages /etc/hosts redirect (--add / --restore) |
| `captures/` | Directory for captured traffic logs |
