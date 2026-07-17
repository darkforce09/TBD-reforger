#!/usr/bin/env bash
# Drive enfusion-mcp over JSON-RPC from the shell. (T-090.0 — hardened)
#
# Daemon-first: reuses a warm enfusion-mcp broker (the Rust `mcpd`, tools/tbd-tools — T-165.7) so calls return in
# ~Workbench round-trip time instead of re-paying the ~35 s index load. Falls back automatically to a
# hardened one-shot run (early-exit consumer → returns at response time, never hangs to the timeout) if
# the daemon is unavailable or MCP_NO_DAEMON=1. Both paths funnel the JSON-RPC id==2 response through
# `cargo xtask mcp consume` (T-162), so result text + exit codes are identical.
#
# Usage:  mcp-call.sh <tool> '<json-args>'        (args default to {})
# Env:    MCP_CALL_TIMEOUT (s, 180)  MCP_CALL_RETRIES (1)  MCP_NO_DAEMON=1  MCP_DEBUG=1  ENFUSION_MCP_BIN  MCP_SOCK
# Exit:   0 success · 1 usage/empty-after-retries · 2 init-failed · 3 JSON-RPC tool error · 4 timeout
set -uo pipefail   # NOT -e: the one-shot early-close intentionally SIGPIPEs the server.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TOOL="${1:-}"
# NB: do NOT write ${2:-{}} — bash matches the first `}` to close the expansion, leaving a literal
# trailing `}` appended to $2 (corrupts args-bearing JSON; the default {} only worked by coincidence).
ARGS="${2:-}"
[ -z "$ARGS" ] && ARGS='{}'
if [ -z "$TOOL" ]; then
  echo "usage: mcp-call.sh <tool> '<json-args>'" >&2
  exit 1
fi

TIMEOUT="${MCP_CALL_TIMEOUT:-180}"
RETRIES="${MCP_CALL_RETRIES:-1}"
# enfusion-mcp needs all three paths to locate the game data + Workbench (mirrors tbd-dev-bootstrap.sh),
# so wb_* tools work even when mcp-call.sh is invoked standalone (not after bootstrap).
export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
export ENFUSION_WORKBENCH_PATH="${ENFUSION_WORKBENCH_PATH:-$HOME/.local/share/Steam/steamapps/common/Arma Reforger Tools}"
export ENFUSION_PROJECT_PATH="${ENFUSION_PROJECT_PATH:-$HOME/Documents/Games/ArmaReforgerWorkbench/addons}"

XTASK="$SCRIPT_DIR/lib/xtask-run.sh"
SOCK="${MCP_SOCK:-${XDG_RUNTIME_DIR:-/tmp}/tbd-mcp-$(id -u).sock}"
[ "${#SOCK}" -gt 100 ] && SOCK="/tmp/tbd-mcp-$(id -u).sock"
export MCP_SOCK="$SOCK"

dbg() { [ "${MCP_DEBUG:-0}" = 1 ] && echo "[mcp-call] $*" >&2 || true; }

TMPS=()
cleanup() { local f; for f in "${TMPS[@]:-}"; do [ -n "$f" ] && rm -f "$f"; done; }
trap cleanup EXIT
mktmp() { local f; f="$(mktemp)"; TMPS+=("$f"); printf '%s' "$f"; }

# --- request framing ---
emit_requests() {
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"cc","version":"1.0"}}}'
  printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"%s","arguments":%s}}\n' "$TOOL" "$ARGS"
}

# --- 4-tier runner resolution (one-shot path) ---
resolve_runner() {
  # T-165.7: .js/.mjs entries run under node; anything else (e.g. the Rust mcpd stub) execs directly.
  if [ -n "${ENFUSION_MCP_BIN:-}" ] && [ -f "$ENFUSION_MCP_BIN" ]; then
    case "$ENFUSION_MCP_BIN" in
      *.js|*.mjs) RUNNER=(node "$ENFUSION_MCP_BIN") ;;
      *)          RUNNER=("$ENFUSION_MCP_BIN") ;;
    esac
    dbg "runner=tier1(ENFUSION_MCP_BIN)"; return
  fi
  local pinned="$SCRIPT_DIR/node_modules/enfusion-mcp/dist/index.js"
  if [ -f "$pinned" ]; then RUNNER=(node "$pinned"); dbg "runner=tier2(pinned)"; return; fi
  local hit; hit="$(find "$HOME/.npm/_npx" -maxdepth 4 -path '*enfusion-mcp/dist/index.js' -type f 2>/dev/null | head -1)"
  if [ -n "$hit" ]; then RUNNER=(node "$hit"); dbg "runner=tier3(npx-cache)"; return; fi
  RUNNER=(npx -y enfusion-mcp); dbg "runner=tier4(npx)"
}

# --- daemon path: 0 success (printed) · 3 tool error · 9 fall-back-to-oneshot ---
ensure_daemon() {
  [ "${MCP_NO_DAEMON:-0}" = 1 ] && return 1
  if bash "$SCRIPT_DIR/mcp-daemon.sh" status >/dev/null 2>&1; then return 0; fi
  if command -v flock >/dev/null 2>&1; then
    local lfd
    exec {lfd}>"$SOCK.lock" 2>/dev/null || true
    [ -n "${lfd:-}" ] && flock -w 65 "$lfd" 2>/dev/null || true
    bash "$SCRIPT_DIR/mcp-daemon.sh" status >/dev/null 2>&1 || bash "$SCRIPT_DIR/mcp-daemon.sh" start >/dev/null 2>&1
    [ -n "${lfd:-}" ] && { exec {lfd}>&- 2>/dev/null || true; }
  else
    bash "$SCRIPT_DIR/mcp-daemon.sh" start >/dev/null 2>&1
  fi
  bash "$SCRIPT_DIR/mcp-daemon.sh" status >/dev/null 2>&1
}

daemon_try() {
  ensure_daemon || { dbg "daemon unavailable"; return 9; }
  dbg "daemon_try TOOL=[$TOOL] ARGS=[$ARGS]"
  local outf errf; outf="$(mktmp)"; errf="$(mktmp)"
  "$XTASK" mcp socket-send "$SOCK" "$TOOL" "$ARGS" 2>"$errf" | "$XTASK" mcp consume >"$outf"
  local send_rc=${PIPESTATUS[0]} consume_rc=${PIPESTATUS[1]}
  dbg "daemon send_rc=$send_rc consume_rc=$consume_rc"
  if [ "$send_rc" = 0 ] && [ "$consume_rc" = 0 ]; then cat "$outf"; return 0; fi
  if [ "$send_rc" = 0 ] && [ "$consume_rc" = 3 ]; then return 3; fi   # consumer already printed the error JSON
  [ "${MCP_DEBUG:-0}" = 1 ] && [ -s "$errf" ] && cat "$errf" >&2
  return 9   # daemon gone / malformed → fall back to one-shot
}

# --- one-shot path with bounded retry on transient {1,2,4} ---
# Stdin is closed right after the request lines so the server hits EOF and exits promptly — the one-shot
# can FAIL FAST (empty) but can NEVER hang. `timeout` is a hard ceiling on the spawned server regardless.
# The early-exit consumer SIGPIPEs the server the instant id==2 arrives, so a good call returns at response
# time. (Reliability + speed for repeated calls comes from the warm daemon; this path is the fallback.)
oneshot() {
  resolve_runner
  local attempt=0 code
  while :; do
    local outf errf; outf="$(mktmp)"; errf="$(mktmp)"
    emit_requests | timeout "$TIMEOUT" "${RUNNER[@]}" 2>"$errf" | "$XTASK" mcp consume >"$outf"
    local to_rc=${PIPESTATUS[1]} consume_rc=${PIPESTATUS[2]}
    dbg "oneshot attempt=$attempt to_rc=$to_rc consume_rc=$consume_rc"
    if [ "$to_rc" = 124 ]; then code=4
    elif [ "$consume_rc" = 0 ]; then cat "$outf"; return 0
    elif [ "$consume_rc" = 3 ]; then return 3   # consumer already printed the error JSON
    elif [ "$consume_rc" = 2 ]; then code=2
    else code=1
    fi
    attempt=$((attempt + 1))
    if [ "$attempt" -gt "$RETRIES" ]; then
      [ -s "$errf" ] && cat "$errf" >&2   # surface server stderr on failure
      return "$code"
    fi
    dbg "retry ($attempt/$RETRIES) after code=$code"
  done
}

rc=9
daemon_try; rc=$?
if [ "$rc" = 9 ]; then oneshot; rc=$?; fi
exit "$rc"
