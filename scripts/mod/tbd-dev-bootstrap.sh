#!/usr/bin/env bash
# TBD dev bootstrap: MCP game root, EMCP handlers, wait for Workbench Net API, connect + validate.
# Usage: bash scripts/mod/tbd-dev-bootstrap.sh [--api] [--server]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
MOD="$MOD_ROOT/tbd-framework"
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
bash "$MOD_SCRIPTS/setup-mcp-game-root.sh"

# Pin enfusion-mcp for the warm MCP daemon / one-shot resolver (tier-2; no npx cold-start).
# Non-fatal offline → mcp-call.sh falls back to the npx cache / `npx -y`.
if [ -f "$MOD_SCRIPTS/package.json" ] && [ ! -d "$MOD_SCRIPTS/node_modules/enfusion-mcp" ]; then
  (cd "$MOD_SCRIPTS" && npm ci --silent) || echo "warn: npm ci in $MOD_SCRIPTS failed (offline?) — using npx-cache fallback"
fi

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
  echo "Then re-run: bash scripts/mod/tbd-dev-bootstrap.sh"
  exit 1
fi

echo "Port $WB_PORT is listening."

# Pre-warm the persistent MCP daemon (one-time ~35 s index load) so wb_connect + all later
# mcp-call.sh invocations are fast. Non-fatal — calls fall back to the one-shot path.
echo "Pre-warming MCP daemon..."
bash "$MOD_SCRIPTS/mcp-daemon.sh" start || echo "warn: daemon pre-warm failed — mcp-call.sh will use one-shot fallback"

bash "$MOD_SCRIPTS/mcp-call.sh" wb_connect '{}' || {
  echo "wb_connect failed — reload tbd-framework addon in Workbench Resource Browser and retry."
  exit 1
}

bash "$MOD_SCRIPTS/mcp-call.sh" mod_validate "{\"modPath\":\"$MOD\"}" || true

for arg in "$@"; do
  case "$arg" in
    --api)
      podman start tbdevent-postgres 2>/dev/null || true
      (cd "$WEB" && npm run dev) &
      echo "API dev server starting on :8080"
      ;;
    --server)
      bash "$MOD_SCRIPTS/setup-server-profile.sh" 2>/dev/null || true
      bash "$MOD_SCRIPTS/run-dev-server.sh" &
      echo "Dedicated server starting..."
      ;;
  esac
done

echo "Bootstrap complete."
