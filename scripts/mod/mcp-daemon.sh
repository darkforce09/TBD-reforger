#!/usr/bin/env bash
# Control the persistent enfusion-mcp broker (T-090.0). Usage: mcp-daemon.sh {start|stop|status|restart}
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOCK="${MCP_SOCK:-${XDG_RUNTIME_DIR:-/tmp}/tbd-mcp-$(id -u).sock}"
# AF_UNIX paths cap at ~108 bytes — fall back to a short /tmp path if the computed one is too long.
[ "${#SOCK}" -gt 100 ] && SOCK="/tmp/tbd-mcp-$(id -u).sock"
PIDFILE="$SOCK.pid"
LOG="$SOCK.log"
DAEMON="$SCRIPT_DIR/lib/mcp-daemon.mjs"

# 4-tier resolve of the enfusion-mcp entry (mirrors mcp-call.sh); echoed for export to the node daemon.
resolve_bin() {
  if [ -n "${ENFUSION_MCP_BIN:-}" ] && [ -f "$ENFUSION_MCP_BIN" ]; then echo "$ENFUSION_MCP_BIN"; return; fi
  local pinned="$SCRIPT_DIR/node_modules/enfusion-mcp/dist/index.js"
  if [ -f "$pinned" ]; then echo "$pinned"; return; fi
  find "$HOME/.npm/_npx" -maxdepth 4 -path '*enfusion-mcp/dist/index.js' -type f 2>/dev/null | head -1
}

# Live check: the socket must be connectable (not just present on disk).
is_running() {
  [ -S "$SOCK" ] || return 1
  python3 - "$SOCK" <<'PY' 2>/dev/null
import socket, sys
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(2)
try:
    s.connect(sys.argv[1])
except OSError:
    sys.exit(1)
s.close()
PY
}

start() {
  if is_running; then echo "mcp-daemon: already running ($SOCK)"; return 0; fi
  [ -e "$SOCK" ] && rm -f "$SOCK"   # stale socket
  local bin; bin="$(resolve_bin)"
  export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
  export ENFUSION_WORKBENCH_PATH="${ENFUSION_WORKBENCH_PATH:-$HOME/.local/share/Steam/steamapps/common/Arma Reforger Tools}"
  export ENFUSION_PROJECT_PATH="${ENFUSION_PROJECT_PATH:-$HOME/Documents/Games/ArmaReforgerWorkbench/addons}"
  [ -n "$bin" ] && export ENFUSION_MCP_BIN="$bin"
  export MCP_SOCK="$SOCK"
  setsid node "$DAEMON" --socket "$SOCK" --pidfile "$PIDFILE" >"$LOG" 2>&1 &
  local i=0
  while [ "$i" -lt 120 ]; do          # up to 60 s (first start pays the one-time index load)
    is_running && { echo "mcp-daemon: started ($SOCK)"; return 0; }
    sleep 0.5; i=$((i + 1))
  done
  echo "mcp-daemon: failed to start (see $LOG)" >&2; return 1
}

stop() {
  [ -f "$PIDFILE" ] && kill "$(cat "$PIDFILE")" 2>/dev/null
  rm -f "$SOCK" "$PIDFILE"
  echo "mcp-daemon: stopped"
}

status() {
  if is_running; then echo "running ($SOCK, pid $(cat "$PIDFILE" 2>/dev/null))"; return 0; fi
  echo "stopped"; return 1
}

# Safety nuke: terminate EVERY tbd MCP broker (any socket), reap orphaned enfusion-mcp servers, and
# clear stray socket/pid/lock/log files. Use if a daemon (or its child) ever leaks.
stop_all() {
  local pids; pids="$(pgrep -f 'lib/mcp-daemon\.mjs' 2>/dev/null || true)"
  if [ -n "$pids" ]; then
    echo "$pids" | xargs -r kill 2>/dev/null        # SIGTERM → daemons kill their own child + unlink
    sleep 1
    echo "$pids" | xargs -r kill -9 2>/dev/null
  fi
  pkill -9 -f 'node_modules/enfusion-mcp/dist/index\.js' 2>/dev/null || true  # reap orphaned servers
  rm -f "${XDG_RUNTIME_DIR:-/tmp}"/tbd-mcp-* /tmp/tbd-mcp-* 2>/dev/null || true
  echo "mcp-daemon: stop-all done"
}

case "${1:-status}" in
  start)    start ;;
  stop)     stop ;;
  restart)  stop; start ;;
  status)   status ;;
  stop-all) stop_all ;;
  *) echo "usage: mcp-daemon.sh {start|stop|status|restart|stop-all}" >&2; exit 2 ;;
esac
