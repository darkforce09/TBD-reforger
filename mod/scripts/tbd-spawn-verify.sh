#!/usr/bin/env bash
# Run Workbench Play and grep logs for slot spawn success.
# Usage: bash scripts/tbd-spawn-verify.sh [slot_id_substring]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PATTERN="${1:-built slot spawn|spawn requested|assigned slot}"

bash "$ROOT/scripts/mcp-call.sh" wb_play '{}' || true
sleep 25
bash "$ROOT/scripts/mcp-call.sh" wb_stop '{}' || true

bash "$ROOT/scripts/mcp-wb-logs.sh" "$PATTERN"
