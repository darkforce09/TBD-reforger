#!/usr/bin/env bash
# Debug Direct Join — writes NDJSON to .cursor/debug-8fc1e0.log (T-162: Rust xtask, no Python)
# Run before AND after attempting Direct Join in Arma.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
XTASK="$SCRIPT_DIR/lib/xtask-run.sh"
DEBUG_LOG="$MONO_ROOT/.cursor/debug-8fc1e0.log"
ENV_FILE="$DEPLOY_ENV"
RUN_ID="${1:-user-repro}"

# shellcheck source=/dev/null
[ -f "$ENV_FILE" ] && source "$ENV_FILE"

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

A2S_PROBE=$("$XTASK" debug a2s-probe --host 192.168.0.140 --ports 2001,17777)

"$XTASK" debug direct-join-log \
  --log "$DEBUG_LOG" \
  --run-id "$RUN_ID" \
  --remote "$REMOTE_OUT" \
  --client-build "$CLIENT_BUILD" \
  --server-build "$SERVER_BUILD" \
  --symlink "$SYMLINK" \
  --ping "$PING" \
  --a2s-json "$A2S_PROBE"

echo "Wrote debug log: $DEBUG_LOG"
echo "--- summary ---"
echo "Client build: $CLIENT_BUILD | Server build: $SERVER_BUILD"
echo "Symlink: $SYMLINK"
echo "$REMOTE_OUT"
