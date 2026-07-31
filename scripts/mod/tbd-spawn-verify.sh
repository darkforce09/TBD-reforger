#!/usr/bin/env bash
# Run Workbench Play and grep logs for slot spawn success.
# Usage:
#   bash scripts/mod/tbd-spawn-verify.sh [extended-grep-pattern]
#   bash scripts/mod/tbd-spawn-verify.sh --selftest    # verdict-logic selftest, no Workbench
#
# T-612 — the old default pattern was `built slot spawn|spawn requested|assigned slot`.
# The first two are DELETED (no `Print` in apps/mod emits either), so on a current log the
# display could only ever show `assigned slot` lines — nothing at all on a headless boot —
# and the verdict underneath (mcp-wb-logs.sh) additionally required `spawn requested` in its
# only exit-0 branch, so this wrapper could not report success on ANY log. The pattern now
# pins tags + event keys, never prose (the rule at remote-log-grep.sh:34); the verdict logic
# and its pattern vocabulary live in mcp-wb-logs.sh — one definition, shared.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"

# The verdict logic under test is mcp-wb-logs.sh's — run ITS selftest without booting anything.
if [ "${1:-}" = "--selftest" ]; then
	exec bash "$MOD_SCRIPTS/mcp-wb-logs.sh" --selftest
fi

# Display filter only — the PASS/PARTIAL/FAIL verdict never depends on this pattern.
PATTERN="${1:-\\[TBD\\]\\[Slots\\]|\\[TBD\\]\\[Loadout\\]|\\[TBD\\]\\[Spawn\\]|assigned slot|bound player}"

bash "$MOD_SCRIPTS/mcp-call.sh" wb_play '{}' || true
sleep 25
bash "$MOD_SCRIPTS/mcp-call.sh" wb_stop '{}' || true

bash "$MOD_SCRIPTS/mcp-wb-logs.sh" "$PATTERN"
