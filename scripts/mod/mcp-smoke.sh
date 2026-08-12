#!/usr/bin/env bash
# Live MCP smoke (T-090.0 gate S1): wb_connect + wb_state must both return non-empty via xtask mcp call.
# Requires a running Workbench with Net API. Exit 0 = OK.
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

fail=0
for tool in wb_connect wb_state; do
  out="$("$SCRIPT_DIR/lib/xtask-run.sh" mcp call "$tool" '{}')"
  rc=$?
  if [ "$rc" = 0 ] && [ -n "$out" ]; then
    echo "mcp-smoke: $tool OK"
  else
    echo "mcp-smoke: $tool FAIL (rc=$rc)" >&2
    fail=1
  fi
done

if [ "$fail" = 0 ]; then
  echo "mcp-smoke: OK"
else
  echo "mcp-smoke: FAIL" >&2
  exit 1
fi
