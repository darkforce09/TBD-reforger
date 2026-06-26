#!/usr/bin/env bash
# Run Workbench Play and grep logs for slot spawn success.
# Usage: bash scripts/mod/tbd-spawn-verify.sh [slot_id_substring]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
PATTERN="${1:-built slot spawn|spawn requested|assigned slot}"

bash "$MOD_SCRIPTS/mcp-call.sh" wb_play '{}' || true
sleep 25
bash "$MOD_SCRIPTS/mcp-call.sh" wb_stop '{}' || true

bash "$MOD_SCRIPTS/mcp-wb-logs.sh" "$PATTERN"
