#!/usr/bin/env bash
# T-181.1 — headless Enfusion compile gate. The "HEMTT for Reforger".
#
# Compiles tbd-framework's Game scripts using the NATIVE Linux dedicated server
# (ArmaReforgerServer) — no Workbench, no Proton, no GPU, no Steam client, no clicking.
# Measured: the compile itself takes ~0.78 s and the whole gate ~1.3 s, versus a 90–120 s
# Workbench restart that also needs a human to click "Open".
#
# Unlike Workbench, this is NOT a serial resource: several runs can execute concurrently
# because each gets its own profile dir.
#
#   bash scripts/mod/compile.sh              # compile the mod, report errors
#   bash scripts/mod/compile.sh --selftest   # ALSO prove the gate still catches a broken file
#   bash scripts/mod/compile.sh --keep-logs  # leave the run dir in place for inspection
#   bash scripts/mod/compile.sh --probe=/tmp/p  # ALSO compile a throwaway addon of .c files —
#                                               # the API-existence oracle, kept OUT of the mod tree
#
# Exit 0 = compiled clean.  Exit 1 = compile errors (printed as file:line).  Exit 2 = harness error.
#
# ── MEASURED FACTS THIS DEPENDS ON (probed 2026-07-25, do not re-derive) ─────────────────
#   * Diagnostics land in  <profile>/logs/logs_<ts>/error.log  as:
#         SCRIPT    (E): @"Scripts/Game/Foo.c,12": Unknown type 'Bar'
#         SCRIPT    (W): @"...": '...' is obsolete: ...
#   * Success markers in console.log:  `Module: Game; loaded NNNNx files`
#                                      `Game successfully created.`
#   * !! The PROCESS EXITS 0 EVEN WHEN COMPILATION FAILS !!  It just never creates the game.
#     So the verdict MUST come from the logs, never from $?.
#   * On success the server keeps running (it goes on to host), so we kill it once we
#     see the success marker.
#   * `int x = ;` compiles CLEAN — Enfusion is lenient. Undefined functions/types are what
#     actually error. That is why --selftest uses an undefined symbol, not bad punctuation.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=../lib/hostrun.sh
source "$ROOT/scripts/lib/hostrun.sh"

MOD_SRC="$ROOT/apps/mod/tbd-framework"
SERVER_DIR="$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server"
SERVER_BIN="$SERVER_DIR/ArmaReforgerServer"
MAX_WAIT="${TBD_COMPILE_TIMEOUT:-180}"

SELFTEST=0
KEEP_LOGS=0
PROBE_DIR=""
for a in "$@"; do
  case "$a" in
    --selftest)  SELFTEST=1 ;;
    --keep-logs) KEEP_LOGS=1 ;;
    --probe=*)   PROBE_DIR="${a#*=}" ;;
    -h|--help)   sed -n '2,30p' "$0"; exit 0 ;;
    *) echo "compile.sh: unknown arg '$a'" >&2; exit 2 ;;
  esac
done

require_host || exit 2

[ -x "$SERVER_BIN" ] || {
  echo "compile.sh: dedicated server not found at:" >&2
  echo "  $SERVER_BIN" >&2
  echo "Install it from Steam (appid 1890870):  steam steam://install/1890870" >&2
  exit 2
}
[ -f "$MOD_SRC/addon.gproj" ] || { echo "compile.sh: no addon.gproj at $MOD_SRC" >&2; exit 2; }

RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/tbd-compile.XXXXXX")"

# Terminate THIS run's server by PID, never by name.
#
# Measured: `pkill -f <pattern>` through the host bridge does NOT kill the server (the
# pattern also matches pkill's own wrapper cmdline, and it self-terminates first), while
# `kill <pid>` through the bridge works reliably. Name-matching is also dangerous here —
# a broad `pkill -f ArmaReforgerServer` would take out a dev/staging server the operator
# started. So the launcher records its own PID into the run dir (shared filesystem) and we
# kill exactly that. Never widen this to a name match.
# The launcher runs under `setsid`, so the recorded PID is a PROCESS GROUP LEADER and we
# signal the whole group with `kill -- -PGID`. Measured why this is necessary: killing only
# the recorded PID reaped `timeout` but left ./ArmaReforgerServer alive as an orphan, which
# leaked one idle server per gate run.
PIDFILE="$RUN_DIR/server.pid"
kill_run() {
  [ -f "$PIDFILE" ] || return 0
  local pgid; pgid="$(cat "$PIDFILE" 2>/dev/null || true)"
  [ -n "$pgid" ] || return 0
  hostrun kill -TERM -- "-$pgid" >/dev/null 2>&1 || true
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    hostrun kill -0 -- "-$pgid" >/dev/null 2>&1 || return 0
    sleep 0.2
  done
  hostrun kill -9 -- "-$pgid" >/dev/null 2>&1 || true
}

cleanup() {
  kill_run
  if [ "$KEEP_LOGS" -eq 1 ]; then
    echo "  (logs kept: $RUN_DIR)"
  else
    rm -rf "$RUN_DIR"
  fi
}
trap cleanup EXIT

mkdir -p "$RUN_DIR/addons" "$RUN_DIR/profile"
# Stage by symlink — the mod tree is the source of truth and is never copied or mutated.
ln -sfn "$MOD_SRC" "$RUN_DIR/addons/tbd-framework"
ADDONS="TBD_Framework"

if [ "$SELFTEST" -eq 1 ]; then
  mkdir -p "$RUN_DIR/addons/tbd-selftest/Scripts/Game"
  cat > "$RUN_DIR/addons/tbd-selftest/addon.gproj" <<'GPROJ'
GameProject {
 ID "TBD_CompileSelfTest"
 GUID "C0FFEE0000000001"
 TITLE "TBD Compile Self Test"
 Dependencies {
  "58D0FB3206B6F859"
 }
 Configurations {
  GameProjectConfig PC {
  }
  GameProjectConfig HEADLESS {
  }
 }
}
GPROJ
  cat > "$RUN_DIR/addons/tbd-selftest/Scripts/Game/TBD_CompileSelfTest.c" <<'BROKEN'
// Deliberately broken — proves the gate still detects compile errors.
// NOTE: must be an undefined symbol; malformed punctuation compiles clean in Enfusion.
class TBD_CompileSelfTest
{
	void Broken()
	{
		TBD_ThisSymbolDoesNotExist_SelfTest();
	}
}
BROKEN
  ADDONS="$ADDONS,TBD_CompileSelfTest"
fi

# --probe=<dir>: stage an arbitrary directory of .c files as a throwaway addon and compile it
# alongside the mod. This is the API-existence oracle: call a suspected Enfusion API, compile,
# read the error. Both wave-1 slice agents independently hand-rolled this, which is why it is
# now first-class. Probes MUST live outside the mod tree (SLICE_WORKFLOW.md) — this makes that
# easy instead of tempting.
if [ -n "$PROBE_DIR" ]; then
  [ -d "$PROBE_DIR" ] || { echo "compile.sh: --probe dir not found: $PROBE_DIR" >&2; exit 2; }
  mkdir -p "$RUN_DIR/addons/tbd-probe/Scripts/Game"
  cat > "$RUN_DIR/addons/tbd-probe/addon.gproj" <<'GPROJ'
GameProject {
 ID "TBD_ApiProbe"
 GUID "C0FFEE0000000002"
 TITLE "TBD API Probe"
 Dependencies {
  "58D0FB3206B6F859"
 }
 Configurations {
  GameProjectConfig PC {
  }
  GameProjectConfig HEADLESS {
  }
 }
}
GPROJ
  cp "$PROBE_DIR"/*.c "$RUN_DIR/addons/tbd-probe/Scripts/Game/" 2>/dev/null || {
    echo "compile.sh: no .c files in $PROBE_DIR" >&2; exit 2; }
  ADDONS="$ADDONS,TBD_ApiProbe"
  echo "    (probing $(ls -1 "$PROBE_DIR"/*.c 2>/dev/null | wc -l) file(s) from $PROBE_DIR)"
fi

echo "==> compiling tbd-framework (native headless server, no Workbench)"
# `setsid` makes the shell a process-group leader and `exec` preserves that PID, so
# $PIDFILE holds the PGID that kill_run() signals. RUN_DIR is on a filesystem both sides see.
hostrun env -C "$SERVER_DIR" setsid sh -c '
  echo $$ > "$1/server.pid"
  exec timeout "$2" ./ArmaReforgerServer \
    -addonsDir "$1/addons" -addons "$3" -profile "$1/profile" -maxFPS 15
' _ "$RUN_DIR" "$MAX_WAIT" "$ADDONS" >"$RUN_DIR/stdout.log" 2>&1 &
SRV_WAIT=$!

# Poll the structured logs — stdout is block-buffered and unreliable.
console=""; errlog=""
deadline=$(( SECONDS + MAX_WAIT ))
verdict=""
while [ $SECONDS -lt $deadline ]; do
  if [ -z "$console" ]; then
    d=$(ls -1d "$RUN_DIR"/profile/logs/logs_* 2>/dev/null | tail -1 || true)
    [ -n "$d" ] && { console="$d/console.log"; errlog="$d/error.log"; }
  fi
  if [ -n "$console" ] && [ -f "$console" ]; then
    if grep -q "Game successfully created" "$console" 2>/dev/null; then verdict=ok;   break; fi
    if grep -q "SCRIPT    (E):"            "$errlog"  2>/dev/null; then verdict=fail; break; fi
  fi
  # NOTE: do NOT infer "died" from $SRV_WAIT here. Under `setsid` the local launcher
  # detaches and exits immediately while the server keeps compiling on the host, so that
  # check reports a false crash. Markers + the deadline are the only trustworthy signals.
  sleep 0.3
done

kill_run
# Give the launcher a moment to notice, but never block on it: when this script runs ON the
# host, $SRV_WAIT is the real `timeout` process and an un-killed `wait` would hang for the
# full MAX_WAIT. The verdict is already decided from the logs at this point.
for _ in 1 2 3 4 5 6 7 8 9 10; do
  kill -0 "$SRV_WAIT" 2>/dev/null || break
  sleep 0.2
done
kill "$SRV_WAIT" 2>/dev/null || true

if [ -z "$verdict" ]; then
  echo "FAIL: timed out after ${MAX_WAIT}s with no compile verdict." >&2
  echo "      (rerun with --keep-logs to inspect)" >&2
  exit 2
fi

if [ "$verdict" = fail ] || { [ -n "$errlog" ] && grep -q "SCRIPT    (E):" "$errlog" 2>/dev/null; }; then
  # `@"path,line": msg`  ->  `path:line: msg`, so editors and humans can jump straight to it.
  all="$RUN_DIR/errors.all"
  sed -n 's/.*SCRIPT    (E): @"\([^"]*\),\([0-9]*\)": \(.*\)/\1:\2: \3/p' "$errlog" | sort -u >"$all"

  # A single broken class aborts the module and cascades into unrelated vanilla files.
  # Those are noise. Partition on ground truth: is the file actually in a staged addon?
  ours="$RUN_DIR/errors.ours"; cascade="$RUN_DIR/errors.cascade"
  : >"$ours"; : >"$cascade"
  while IFS= read -r line; do
    p="${line%%:*}"
    if [ -e "$MOD_SRC/$p" ] || [ -e "$RUN_DIR/addons/tbd-selftest/$p" ] \
       || [ -e "$RUN_DIR/addons/tbd-probe/$p" ]; then
      echo "$line" >>"$ours"
    else
      echo "$line" >>"$cascade"
    fi
  done <"$all"

  echo
  echo "FAIL: Enfusion compile errors"
  echo "------------------------------------------------------------"
  if [ -s "$ours" ]; then
    cat "$ours"
  else
    echo "(none in TBD sources — see cascade below; the root cause may be a"
    echo " missing dependency or a vanilla API that moved)"
  fi
  echo "------------------------------------------------------------"
  echo "$(wc -l <"$ours") error(s) in TBD sources, $(wc -l <"$cascade") cascaded into vanilla."
  if [ -s "$cascade" ]; then
    echo "Cascade (fix the TBD errors first; these usually vanish):"
    sed 's/^/  /' "$cascade" | head -10
    [ "$(wc -l <"$cascade")" -gt 10 ] && echo "  … $(( $(wc -l <"$cascade") - 10 )) more"
  fi
  exit 1
fi

# ── LOAD-COUNT GUARD ────────────────────────────────────────────────────────────────────
# Measured (T-181.7): if resourceDatabase.rdb is missing or stale, the engine silently skips
# script compilation for every LOOSE addon — the Game module falls back to vanilla-only — and
# this gate still reported "compiled clean". A green gate that is not compiling our code is
# worse than no gate. (A canary addon cannot detect this: it dies with the mod. Verified.)
#
# The check is EXACT rather than a fudge factor: measure the vanilla-only count once, then
# require strictly more than that. Any TBD script at all pushes the count above vanilla; the
# silent-failure mode lands exactly ON it. An earlier version compared against a ratcheting
# best-seen count with an 80%-of-.c-files threshold — the wave-1 verifier proved that fired
# with a margin of ONE file and would have silently stopped working after two more
# WorkbenchGame scripts (which never enter the Game module), and that a fresh clone could
# record a BROKEN count as its baseline.
VANILLA_BASELINE_FILE="$ROOT/.compile-vanilla-baseline"
loaded=$(grep -o "Module: Game; loaded [0-9]*x files" "$console" | tail -1 | grep -o "[0-9]*" || echo 0)
if [ ! -s "$VANILLA_BASELINE_FILE" ]; then
  echo "    (calibrating vanilla-only baseline, one time)"
  cal_dir="$(mktemp -d "${TMPDIR:-/tmp}/tbd-cal.XXXXXX")"
  mkdir -p "$cal_dir/addons" "$cal_dir/profile"
  hostrun env -C "$SERVER_DIR" setsid sh -c '
    echo $$ > "$1/server.pid"
    exec timeout 120 ./ArmaReforgerServer -addonsDir "$1/addons" -profile "$1/profile" -maxFPS 15
  ' _ "$cal_dir" >/dev/null 2>&1 &
  cal_pid=$!
  cal_console=""
  cal_deadline=$(( SECONDS + 120 ))
  while [ $SECONDS -lt $cal_deadline ]; do
    [ -z "$cal_console" ] && { d=$(ls -1d "$cal_dir"/profile/logs/logs_* 2>/dev/null | tail -1 || true); [ -n "$d" ] && cal_console="$d/console.log"; }
    [ -n "$cal_console" ] && grep -q "Module: Game; loaded" "$cal_console" 2>/dev/null && break
    sleep 0.3
  done
  cal_n=$(grep -o "Module: Game; loaded [0-9]*x files" "$cal_console" 2>/dev/null | tail -1 | grep -o "[0-9]*" || echo 0)
  [ "${cal_n:-0}" -gt 0 ] && echo "$cal_n" > "$VANILLA_BASELINE_FILE"
  [ -f "$cal_dir/server.pid" ] && hostrun kill -9 -- "-$(cat "$cal_dir/server.pid")" >/dev/null 2>&1 || true
  kill "$cal_pid" 2>/dev/null || true
  rm -rf "$cal_dir"
fi
vanilla=$(cat "$VANILLA_BASELINE_FILE" 2>/dev/null || echo 0)
if [ "${vanilla:-0}" -gt 0 ] && [ "${loaded:-0}" -le "${vanilla:-0}" ]; then
  echo
  echo "FAIL: the mod's scripts did not compile."
  echo "  Game module loaded $loaded files; vanilla-only is $vanilla."
  echo "  Loading no more than vanilla means tbd-framework was NOT compiled, even though the"
  echo "  run itself was clean. Most likely cause: resourceDatabase.rdb missing or stale."
  echo "  Fix: open apps/mod/tbd-framework in Workbench once so it regenerates the rdb."
  exit 1
fi

files=$(grep -o "Module: Game; loaded [0-9]*x files; [0-9]*x classes" "$console" | tail -1 || true)
took=$(grep -o "Compiling Game scripts took: [0-9.]* ms" "$console" | tail -1 || true)
warn=$(grep -c "SCRIPT    (W): @\"Scripts/Game/TBD/" "$errlog" 2>/dev/null || echo 0)
echo "OK: compiled clean"
[ -n "$files" ] && echo "    $files"
[ -n "$took" ]  && echo "    $took"
echo "    $warn warning(s) in TBD sources"
exit 0
