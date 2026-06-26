#!/usr/bin/env bash
# Debug Direct Join — writes NDJSON to .cursor/debug-8fc1e0.log
# Run before AND after attempting Direct Join in Arma.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
DEBUG_LOG="$MONO_ROOT/.cursor/debug-8fc1e0.log"
ENV_FILE="$DEPLOY_ENV"
RUN_ID="${1:-user-repro}"

# shellcheck source=/dev/null
[ -f "$ENV_FILE" ] && source "$ENV_FILE"

log_json() {
  local hid="$1" msg="$2" data="$3"
  python3 -c "
import json, time
print(json.dumps({
  'sessionId': '8fc1e0',
  'timestamp': int(time.time()*1000),
  'location': 'scripts/debug-direct-join.sh',
  'message': '$msg',
  'data': json.loads('$data'),
  'hypothesisId': '$hid',
  'runId': '$RUN_ID'
}))
" >> "$DEBUG_LOG"
}

PATH="$HOME/.local/bin:$PATH"
SSH_BASE=(ssh -o StrictHostKeyChecking=no)
[ -n "${TBD_SSH_PASS:-}" ] && SSH_BASE=(sshpass -p "$TBD_SSH_PASS" ssh -o StrictHostKeyChecking=no)

CLIENT_BUILD=$(grep buildid "$HOME/.local/share/Steam/steamapps/appmanifest_1874880.acf" 2>/dev/null | awk '{print $3}' | tr -d '"' || echo unknown)
SERVER_BUILD=$(grep buildid "$HOME/.local/share/Steam/steamapps/appmanifest_1874900.acf" 2>/dev/null | awk '{print $3}' | tr -d '"' || echo unknown)
SYMLINK=$(readlink -f "$HOME/.local/share/tbd-server-addons/tbd-framework" 2>/dev/null || echo missing)

REMOTE_OUT=""
if [ -n "${TBD_SSH_HOST:-}" ]; then
  REMOTE_OUT=$("${SSH_BASE[@]}" "$TBD_SSH_HOST" 'bash -s' <<'RS' 2>/dev/null || true
SVC=$(systemctl --user is-active tbd-reforger.service 2>/dev/null || echo inactive)
P2001=$(ss -ulnp 2>/dev/null | grep -c ':2001 ' || echo 0)
P17777=$(ss -ulnp 2>/dev/null | grep -c ':17777 ' || echo 0)
LOG=$(ls -td /home/sam/tbd/profile/logs/logs_* 2>/dev/null | head -1)/console.log
LISTEN=$(grep "listening on address" "$LOG" 2>/dev/null | tail -1 || echo none)
A2S=$(grep -i A2S "$LOG" 2>/dev/null | tail -2 || echo none)
CLIENT=$(grep -iE "connect|client|join|session" "$LOG" 2>/dev/null | tail -3 || echo none)
echo "service=$SVC udp2001=$P2001 udp17777=$P17777"
echo "listen=$LISTEN"
echo "a2s=$A2S"
echo "client_lines=$CLIENT"
RS
)
fi

PING=$(ping -c 1 -W 2 192.168.0.140 2>&1 | grep -oP 'time=\K[0-9.]+' || echo fail)

A2S_PROBE=$(python3 - <<'PY'
import json, socket
def probe(port):
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.settimeout(2)
    try:
        s.sendto(b'\xFF\xFF\xFF\xFFTSource Engine Query\x00', ('192.168.0.140', port))
        data, addr = s.recvfrom(4096)
        return {"port": port, "ok": True, "bytes": len(data), "from": str(addr)}
    except Exception as e:
        return {"port": port, "ok": False, "error": str(e)}
    finally:
        s.close()
print(json.dumps({"p2001": probe(2001), "p17777": probe(17777)}))
PY
)

python3 <<PY
import json, time
LOG = "$DEBUG_LOG"
def w(hid, msg, data, run_id="$RUN_ID"):
    with open(LOG, "a") as f:
        f.write(json.dumps({
            "sessionId": "8fc1e0",
            "timestamp": int(time.time()*1000),
            "location": "scripts/debug-direct-join.sh",
            "message": msg,
            "data": data,
            "hypothesisId": hid,
            "runId": run_id
        }) + "\n")

remote = """$REMOTE_OUT""".strip()
a2s_probe = json.loads("""$A2S_PROBE""")
w("H1", "remote service and ports", {
    "raw": remote,
    "service_active": "service=active" in remote,
    "udp_2001": "udp2001=1" in remote or "udp2001=2" in remote,
    "udp_17777": "udp17777=1" in remote,
})
w("H2", "steam build ids", {
    "client_build": "$CLIENT_BUILD",
    "server_build": "$SERVER_BUILD",
    "builds_match": "$CLIENT_BUILD" == "$SERVER_BUILD" and "$CLIENT_BUILD" != "unknown",
})
w("H3", "a2s and listen log", {"remote_snippet": remote})
w("H4", "ping", {"ms": "$PING", "host": "192.168.0.140"})
w("H5", "client mod symlink", {"path": "$SYMLINK", "exists": "$SYMLINK" != "missing"})
w("H6", "a2s port probe from client PC", a2s_probe)
PY

echo "Wrote debug log: $DEBUG_LOG"
echo "--- summary ---"
echo "Client build: $CLIENT_BUILD | Server build: $SERVER_BUILD"
echo "Symlink: $SYMLINK"
echo "$REMOTE_OUT"
