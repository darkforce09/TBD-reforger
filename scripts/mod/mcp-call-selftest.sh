#!/usr/bin/env bash
# Offline self-test for the MCP call path (T-090.0 gates T1-T7; T-162 Rust consumer).
# No Workbench / no real enfusion-mcp: drives `xtask mcp consume` against recorded fixtures
# and mcp-call.sh against the Rust mcpd stub (MCP_STUB=1 — T-165.7).
# Exit 0 iff every gate passes.
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
XTASK="$SCRIPT_DIR/lib/xtask-run.sh"
# T-165.7: the stub is the Rust mcpd in stub mode (MCP_STUB=1; broker mode still wins on
# --socket, so exporting it around mcp-daemon.sh is safe).
STUB="$("$SCRIPT_DIR/lib/mcpd-bin.sh")"
export MCP_STUB=1
FIX="$SCRIPT_DIR/fixtures"
SOCK="/tmp/tbd-mcp-selftest-$(id -u).sock"

PASS=0; FAIL=0
ok() { echo "  ✓ $1"; PASS=$((PASS + 1)); }
no() { echo "  ✗ $1" >&2; FAIL=$((FAIL + 1)); }
rc_is() { [ "$2" = "$3" ] && ok "$1 (rc=$3)" || no "$1 (want rc$2 got rc$3)"; }

cleanup() { MCP_SOCK="$SOCK" bash "$SCRIPT_DIR/mcp-daemon.sh" stop >/dev/null 2>&1; rm -f "$SOCK"*; }
trap cleanup EXIT
cleanup

echo "[T2-T5] consumer fixtures"
out=$("$XTASK" mcp consume < "$FIX/mcp-wb-state-success.jsonl"); rc=$?
{ [ "$rc" = 0 ] && [ -n "$out" ]; } && ok "T2 success rc0 non-empty" || no "T2 success (rc=$rc out=[$out])"
"$XTASK" mcp consume < "$FIX/mcp-tool-error.jsonl" 1>/dev/null 2>/tmp/.st_e; rc_is "T3 error (rpc)" 3 $?
grep -q '"code"' /tmp/.st_e && ok "T3 error JSON on stderr" || no "T3 error JSON missing"
"$XTASK" mcp consume < "$FIX/mcp-tool-iserror.jsonl" 1>/dev/null 2>/tmp/.st_e; rc_is "T3b error (isError)" 3 $?
grep -q 'MCP error' /tmp/.st_e && ok "T3b isError text on stderr" || no "T3b isError text missing"
"$XTASK" mcp consume < "$FIX/mcp-init-fail.jsonl" >/dev/null 2>&1; rc_is "T4 init-fail" 2 $?
out=$("$XTASK" mcp consume < "$FIX/mcp-empty.jsonl"); rc=$?
{ [ "$rc" = 1 ] && [ -z "$out" ]; } && ok "T5 empty rc1 empty-out" || no "T5 empty (rc=$rc out=[$out])"

echo "[T6] usage error, no spawn"
bash "$SCRIPT_DIR/mcp-call.sh" >/dev/null 2>/tmp/.st_e; rc_is "T6 usage" 1 $?
grep -q usage /tmp/.st_e && ok "T6 usage text on stderr" || no "T6 usage text missing"

echo "[one-shot wrapper via stub] (MCP_NO_DAEMON=1)"
osh() { MCP_NO_DAEMON=1 ENFUSION_MCP_BIN="$STUB" bash "$SCRIPT_DIR/mcp-call.sh" wb_state '{}'; }
out=$(STUB_MODE=success STUB_LINGER=0.3 osh 2>/dev/null); rc=$?
{ [ "$rc" = 0 ] && [ "$out" = "STUB-OK wb_state edit 123" ]; } && ok "one-shot success rc0" || no "one-shot success (rc=$rc out=[$out])"
STUB_MODE=error STUB_LINGER=0.3 osh >/dev/null 2>/dev/null; rc_is "one-shot error" 3 $?
STUB_MODE=initfail STUB_LINGER=0.3 MCP_CALL_RETRIES=0 osh >/dev/null 2>/dev/null; rc_is "one-shot init-fail" 2 $?
STUB_MODE=empty STUB_LINGER=0.3 MCP_CALL_RETRIES=1 osh >/dev/null 2>/tmp/.st_e; rc_is "one-shot empty+retry" 1 $?
grep -q STUB-STDERR-MARKER /tmp/.st_e && ok "T7 stderr surfaced on failure" || no "T7 stderr not surfaced"
STUB_MODE=empty STUB_LINGER=4 MCP_CALL_TIMEOUT=1 MCP_CALL_RETRIES=0 osh >/dev/null 2>/dev/null; rc_is "one-shot timeout" 4 $?

echo "[daemon via stub-daemon] (short socket, offline)"
# Short idle + max-life so a test daemon self-reaps within seconds even if cleanup is skipped.
export MCP_SOCK="$SOCK" MCP_DAEMON_IDLE=8 MCP_DAEMON_MAX_LIFE=30
ENFUSION_MCP_BIN="$STUB" STUB_DAEMON=1 bash "$SCRIPT_DIR/mcp-daemon.sh" start >/dev/null 2>&1
MCP_SOCK="$SOCK" bash "$SCRIPT_DIR/mcp-daemon.sh" status >/dev/null 2>&1; rc_is "daemon start+status" 0 $?
out=$(ENFUSION_MCP_BIN="$STUB" STUB_DAEMON=1 bash "$SCRIPT_DIR/mcp-call.sh" wb_state '{}' 2>/dev/null); rc=$?
{ [ "$rc" = 0 ] && [ "$out" = "STUB-DAEMON-OK wb_state args={}" ]; } && ok "daemon call rc0" || no "daemon call (rc=$rc out=[$out])"
# regression: args-bearing call must round-trip uncorrupted (guards the ${2:-{}} extra-brace bug)
out=$(ENFUSION_MCP_BIN="$STUB" STUB_DAEMON=1 bash "$SCRIPT_DIR/mcp-call.sh" api_search '{"query":"Ztest"}' 2>/dev/null)
[ "$out" = 'STUB-DAEMON-OK api_search args={"query":"Ztest"}' ] && ok "args round-trip (no brace corruption)" || no "args round-trip (out=[$out])"
cleanup
out=$(MCP_NO_DAEMON=1 ENFUSION_MCP_BIN="$STUB" STUB_MODE=success STUB_LINGER=0.3 bash "$SCRIPT_DIR/mcp-call.sh" wb_state '{}' 2>/dev/null); rc=$?
{ [ "$rc" = 0 ] && [ -n "$out" ]; } && ok "fallback when no daemon" || no "fallback (rc=$rc out=[$out])"

rm -f /tmp/.st_e
echo "---"
if [ "$FAIL" = 0 ]; then echo "mcp-call-selftest: ALL PASS ($PASS)"; exit 0; fi
echo "mcp-call-selftest: FAIL ($FAIL failed, $PASS passed)" >&2; exit 1
