#!/usr/bin/env bash
# TBD dev bootstrap: MCP game root, EMCP handlers, wait for Workbench Net API, connect + validate.
# Usage: bash scripts/tbd-dev-bootstrap.sh [--api] [--server]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MOD="$ROOT/tbd-framework"
GPROJ="$MOD/addon.gproj"
MCP_MOD="/home/Samuel/.npm/_npx/be402e1c82700767/node_modules/enfusion-mcp/mod"
HANDLERS_SRC="$MCP_MOD/Scripts/WorkbenchGame/EnfusionMCP"
HANDLERS_DST="$MOD/Scripts/WorkbenchGame/EnfusionMCP"
WB_PORT="${ENFUSION_WORKBENCH_PORT:-5775}"
WAIT_SEC="${TBD_WB_WAIT_SEC:-180}"

export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
export ENFUSION_WORKBENCH_PATH="${ENFUSION_WORKBENCH_PATH:-$HOME/.local/share/Steam/steamapps/common/Arma Reforger Tools}"
export ENFUSION_PROJECT_PATH="${ENFUSION_PROJECT_PATH:-$HOME/Documents/Games/ArmaReforgerWorkbench/addons}"

echo "== TBD dev bootstrap =="
bash "$ROOT/scripts/setup-mcp-game-root.sh"

if [ -d "$HANDLERS_SRC" ] && [ ! -d "$HANDLERS_DST" ]; then
  mkdir -p "$(dirname "$HANDLERS_DST")"
  cp -a "$HANDLERS_SRC" "$HANDLERS_DST"
  echo "Installed EMCP handlers to $HANDLERS_DST"
fi

port_open() {
  ss -tln 2>/dev/null | grep -q ":${WB_PORT} " || netstat -tln 2>/dev/null | grep -q ":${WB_PORT} "
}

if ! port_open; then
  echo "Workbench Net API not on :${WB_PORT} — trying steam -applaunch 1874910 ..."
  steam -applaunch 1874910 2>/dev/null || true
  elapsed=0
  while ! port_open && [ "$elapsed" -lt "$WAIT_SEC" ]; do
    sleep 3
    elapsed=$((elapsed + 3))
  done
fi

if ! port_open; then
  echo ""
  echo "ACTION REQUIRED: Launch Arma Reforger Tools from Steam, open $GPROJ, enable Net API (File > Options > General)."
  echo "Then re-run: bash scripts/tbd-dev-bootstrap.sh"
  exit 1
fi

echo "Port $WB_PORT is listening."
bash "$ROOT/scripts/mcp-call.sh" wb_connect '{}' || {
  echo "wb_connect failed — reload tbd-framework addon in Workbench Resource Browser and retry."
  exit 1
}

bash "$ROOT/scripts/mcp-call.sh" mod_validate "{\"modPath\":\"$MOD\"}" || true

for arg in "$@"; do
  case "$arg" in
    --api)
      podman start tbdevent-postgres 2>/dev/null || true
      (cd "$ROOT/Tbdevent_Website" && npm run dev) &
      echo "API dev server starting on :8080"
      ;;
    --server)
      bash "$ROOT/scripts/setup-server-profile.sh" 2>/dev/null || true
      bash "$ROOT/scripts/run-dev-server.sh" &
      echo "Dedicated server starting..."
      ;;
  esac
done

echo "Bootstrap complete."
