#!/usr/bin/env bash
# Grep the latest Workbench Play console.log for TBD spawn diagnostics.
# Run after MCP wb_play (and optional sleep) — enfusion-mcp has no wb_log tool.
# Usage: mcp-wb-logs.sh [extended-grep-pattern]
set -euo pipefail

PATTERN="${1:-\\[TBD\\]|SpawnLogic|assigned slot|built slot spawn|Stage →}"
PROTON_LOG_DIR="$HOME/.local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/logs"
NATIVE_LOG_DIR="$HOME/Documents/Games/ArmaReforgerWorkbench/logs"

latest_log_dir() {
  local d picked=""
  for d in "$PROTON_LOG_DIR" "$NATIVE_LOG_DIR"; do
    [ -d "$d" ] || continue
    picked="$(ls -td "$d"/logs_* 2>/dev/null | head -1)"
    [ -n "$picked" ] && echo "$picked" && return
  done
}

LATEST="$(latest_log_dir)"
if [ -z "$LATEST" ]; then
  echo "No Workbench log directory found."
  exit 1
fi

LOG="$LATEST/console.log"
echo "Log: $LOG"
echo "---"

if ! grep -E "$PATTERN" "$LOG" 2>/dev/null; then
  echo "(no lines matched: $PATTERN)"
  exit 1
fi

echo "---"
if grep -q "SpawnManager: assigned slot" "$LOG" && grep -q "spawn requested" "$LOG"; then
  echo "PASS: assigned slot + spawn requested found."
  exit 0
fi

if grep -q "built slot spawn" "$LOG"; then
  echo "PARTIAL: slot spawn points built; deploy not confirmed yet."
  exit 2
fi

echo "FAIL: expected TBD spawn lines missing."
exit 1
