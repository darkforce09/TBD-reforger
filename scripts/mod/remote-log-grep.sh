#!/usr/bin/env bash
# Grep TBD spawn lines from the dedicated server log on 192.168.0.140 via SSH.
# Usage: bash scripts/mod/remote-log-grep.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
ENV_FILE="$DEPLOY_ENV"
# shellcheck source=/dev/null
[ -f "$ENV_FILE" ] && source "$ENV_FILE"

: "${TBD_SSH_HOST:?Set TBD_SSH_HOST in scripts/deploy/deploy.env}"
: "${TBD_PROFILE_DIR:?Set TBD_PROFILE_DIR in scripts/deploy/deploy.env}"

ssh_cmd() {
  if [ -n "${TBD_SSH_PASS:-}" ]; then
    sshpass -p "$TBD_SSH_PASS" ssh -o StrictHostKeyChecking=no "$TBD_SSH_HOST" "$@"
  elif [ -n "${TBD_SSH_IDENTITY_FILE:-}" ]; then
    ssh -i "$TBD_SSH_IDENTITY_FILE" -o StrictHostKeyChecking=no "$TBD_SSH_HOST" "$@"
  else
    ssh -o StrictHostKeyChecking=no "$TBD_SSH_HOST" "$@"
  fi
}

FIND_LOG="
ls -td '$TBD_PROFILE_DIR'/logs/logs_* '$TBD_PROFILE_DIR'/profile/logs/logs_* 2>/dev/null | while read -r d; do
  [ -f \"\$d/console.log\" ] && echo \"\$d/console.log\" && exit 0
done
exit 1
"

LOG="$(ssh_cmd "bash -lc $(printf '%q' "$FIND_LOG")" 2>/dev/null || true)"
if [ -z "$LOG" ]; then
  echo "No console.log found under $TBD_PROFILE_DIR (logs/ or profile/logs/)."
  exit 1
fi

echo "Remote log: $LOG"
echo "---"

PATTERN='\[TBD\]|built slot spawn|assigned slot|spawn requested|Can.t compile|RequestSpawn failed|Unknown class'
ssh_cmd "grep -E \"$PATTERN\" \"$LOG\" 2>/dev/null | tail -80" || true

echo "---"
FAIL=0
for need in "Mission loaded" "built slot spawn" "Stage → LOBBY"; do
  if ! ssh_cmd "grep -q \"$need\" \"$LOG\" 2>/dev/null"; then
    echo "MISSING: $need"
    FAIL=1
  fi
done

if ssh_cmd "grep -qE 'Can.t compile|Unknown class .TBD_SpawnLogic' \"$LOG\" 2>/dev/null"; then
  echo "FAIL: compile or spawn logic errors in log"
  FAIL=1
fi

if ssh_cmd "grep -q 'SpawnManager: assigned slot' \"$LOG\" && grep -q 'spawn requested' \"$LOG\""; then
  echo "PASS: assigned slot + spawn requested found."
  exit 0
fi

if ssh_cmd "grep -q 'built slot spawn' \"$LOG\""; then
  echo "PARTIAL: slot spawn points built; no player deploy in log yet (join a client?)."
  exit 2
fi

if [ "$FAIL" -eq 1 ]; then
  exit 1
fi

echo "FAIL: expected TBD spawn lines missing."
exit 1
