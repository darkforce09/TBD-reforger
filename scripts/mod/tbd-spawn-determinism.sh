#!/usr/bin/env bash
# Spawn/equip determinism gate (T-068 follow-up program; Makefile: make mod-spawn-determinism).
# Hub: docs/mod/SPAWN_DETERMINISM.md · verify log: .ai/artifacts/spawn_determinism_verify_log.md
#
# Runs N Workbench play sessions of the currently open world and asserts the
# spawn/equip outcome is IDENTICAL across every run:
#   1. normalized [TBD] digests identical across runs
#   2. zero `path=vanilla-fallthrough`
#   3. settle census characters == bodies (when the [TBD][Audit] line exists)
#   4. every issued gear line ends `equip OK` / `swapped` / `swap-skipped`
#   5. exactly one bind per player per run; materialization + census present
#   6. zero SCRIPT (E) / Virtual Machine Exception lines
#
# The Workbench is RESTARTED between runs: TBD_MissionLoader/RosterLoader statics
# survive play sessions inside one WB process (measured, T-068.12 verify log), so
# a same-process re-play does not exercise the fetch/settle path.
#
# Usage:
#   tbd-spawn-determinism.sh --preflight
#   tbd-spawn-determinism.sh [N-runs (default 5)] [world (default worlds/TBD_Dev_POC.ent)]
# Env: TBD_DET_KEEP=1 keeps per-run snapshots; TBD_DET_TIMEOUT (default 120) per-run seconds.
#
# Prerequisite: live Arma Reforger Workbench with Net API on :5775 (or
# ENFUSION_WORKBENCH_PORT). Cannot run headless / in CI — preflight fails fast
# (exit 2) with an actionable message instead of waiting minutes for a relaunch.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/gate-grep.sh
source "$SCRIPT_DIR/lib/gate-grep.sh"
WB_PORT="${ENFUSION_WORKBENCH_PORT:-5775}"

PROTON_LOG_DIR="$HOME/.local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/logs"
NATIVE_LOG_DIR="$HOME/Documents/Games/ArmaReforgerWorkbench/logs"

mcp() { bash "$SCRIPT_DIR/mcp-call.sh" "$@"; }

latest_log() {
  local d picked=""
  for d in "$PROTON_LOG_DIR" "$NATIVE_LOG_DIR"; do
    [ -d "$d" ] || continue
    picked="$(ls -td "$d"/logs_* 2>/dev/null | head -1)"
    [ -n "$picked" ] && { echo "$picked/console.log"; return; }
  done
}

port_open() { ss -tln 2>/dev/null | grep -q ":${WB_PORT} "; }

# Fail in seconds when Workbench cannot run here (headless CI, no Steam session).
# Does not launch Steam — that is the operator's job before make mod-spawn-determinism.
preflight() {
  if port_open; then
    echo "preflight: Workbench Net API listening on :$WB_PORT"
    return 0
  fi
  cat >&2 <<EOF
FATAL: Workbench Net API not listening on :$WB_PORT — spawn-determinism cannot run.
  Prerequisite: Arma Reforger Workbench with Net API enabled on this host.
  Start Workbench (e.g. steam -applaunch 1874910), wait until :$WB_PORT is up,
  then: make mod-spawn-determinism
  Docs: docs/mod/SPAWN_DETERMINISM.md
  This gate is NOT headless and is NOT part of make ci-local / wave.sh gates.
  Offline MCP (no Workbench): make mcp-selftest
EOF
  return 2
}

if [ "${1:-}" = "--preflight" ]; then
  preflight
  exit $?
fi

RUNS="${1:-5}"
WORLD="${2:-worlds/TBD_Dev_POC.ent}"
TIMEOUT="${TBD_DET_TIMEOUT:-120}"
OUT_DIR="$(mktemp -d /tmp/tbd-spawn-det.XXXXXX)"

preflight || exit $?

# One kill→relaunch cycle. Returns nonzero when the instance came up in the
# "Can't initialize the game" state (Net API answers but the World Editor is dead
# — happens when the relaunch races Steam), so the caller can cycle again.
restart_wb_once() {
  # Bracket trick: the pattern must not match THIS script's own cmdline.
  pkill -f "WorkbenchSteamD[i]ag" 2>/dev/null
  sleep 5
  port_open || steam -applaunch 1874910 2>/dev/null || true
  local waited=0
  until port_open; do
    sleep 5; waited=$((waited + 5))
    [ "$waited" -ge 300 ] && { echo "FATAL: Workbench did not come back on :$WB_PORT"; exit 2; }
  done
  sleep 15
  mcp wb_connect '{}' >/dev/null 2>&1 || return 1
  # A game-dead instance accepts connections but cannot open worlds — probe it.
  mcp wb_open_resource "{\"path\":\"$WORLD\"}" 2>&1 | grep -qE "Resource Opened|Opened:" || return 1
  sleep 15
  return 0
}

restart_wb() {
  local try
  for try in 1 2 3; do
    restart_wb_once && return 0
    echo "WARN: Workbench came up game-dead (try $try) — cycling again"
  done
  echo "FATAL: Workbench game-dead after 3 restart cycles"
  exit 2
}

# Normalize to the OUTCOME set so run digests compare what players experience:
# volatile tokens stripped; `equip OK` and `swap-skipped (already worn)` merge to
# one `gear-ensured` token (the vanilla double-spawn hands over a dressed OR naked
# second body nondeterministically — measured — and the reap converges the end
# state either way); `sort -u` collapses the duplicate wave lines. Multiplicity
# is asserted separately (one spawn request per player, census, zero errors).
normalize() {
  sed -E \
    -e 's/^[0-9:.]+[[:space:]]+//' \
    -e 's/SCRIPT[[:space:]]*(\((E|W)\))?[[:space:]]*:[[:space:]]*//' \
    -e 's/0x[0-9A-Fa-f]+/0xID/g' \
    -e 's/ent=[^ ]+/ent=ID/g' \
    -e 's/weapon=[^ ]+/weapon=ID/g' \
    -e 's/<[-0-9., ]+>/<POS>/g' \
    -e 's/\([-0-9.,]+\)/(POS)/g' \
    -e 's/(feetY|surfaceY|groundDelta|yaw|Y|delta)=[-0-9.e]+/\1=N/g' \
    -e 's/took: [0-9.]+ ms/took: N ms/g' \
    -e 's/^(\[TBD\]\[Loadout\]\[[A-Za-z]+\]) ([a-z]+) (swap-skipped \(already worn\)|equip OK) (\{[0-9A-F]+\}[^ ]+).*/\1 GEAR-ENSURED \2 \4/' \
  | LC_ALL=C sort -u
}

# Digest = the PLAYER-VISIBLE outcome set. Excluded as diagnostics (their presence
# encodes vanilla-internal jitter, all still asserted elsewhere / kept in raw):
#  - "application cancelled" / "reaping superseded": whether an equip wave was
#    in flight at reap time is sub-second vanilla timing;
#  - "deployed player=" transform logs: the 500 ms diagnostic can win or lose the
#    race against the reap;
#  - "swapped area=": vanilla RANDOMIZES kit cosmetic variants per spawn (measured:
#    Jacket_US_BDU vs _rolledup), so swap-vs-skip flips while the final worn item
#    (GEAR-ENSURED line) is identical.
extract() { grep -E "\[TBD\]\[Spawn\]|\[TBD\]\[Slots\]|\[TBD\]\[Loadout\]|\[TBD\]\[Audit\]|Stage →|bound player|assigned slot|Mission loaded|Roster" "$1" | grep -vE "application cancelled|deployed player=|swapped area=" ; }

fail=0
declare -a DIGESTS

for i in $(seq 1 "$RUNS"); do
  echo "── run $i/$RUNS ──"
  restart_wb
  LOG="$(latest_log)"
  [ -n "$LOG" ] || { echo "FATAL: no console.log found"; exit 2; }
  MARK=$(wc -l < "$LOG" 2>/dev/null || echo 0)

  mcp wb_play '{}' >/dev/null || { echo "FATAL: wb_play failed"; exit 2; }

  waited=0 done_flag=0
  while [ "$waited" -lt "$TIMEOUT" ]; do
    sleep 5; waited=$((waited + 5))
    NEW="$(tail -n +"$((MARK + 1))" "$LOG" 2>/dev/null)"
    if echo "$NEW" | grep -qE "\[TBD\]\[Audit\]"; then done_flag=1; break; fi
  done
  # settle a moment for trailing lines (census fires after the pass line)
  sleep 8
  NEW="$(tail -n +"$((MARK + 1))" "$LOG" 2>/dev/null)"
  mcp wb_stop '{}' >/dev/null

  if [ "$done_flag" -ne 1 ]; then
    echo "FAIL run $i: sentinel not seen within ${TIMEOUT}s"
    fail=1
  fi

  RAW="$OUT_DIR/run$i.raw.log"; NORM="$OUT_DIR/run$i.norm.log"
  echo "$NEW" > "$RAW"
  echo "$NEW" | extract /dev/stdin | normalize > "$NORM" || true

  # per-run assertions
  #
  # T-556: these two are BANS — the thing being searched for is the failure — so the
  # `if grep …; then fail=1; fi` shape reported a clean run for three different reasons
  # and could only tell one of them apart: no match (genuinely clean), the log file
  # missing (grep 2), and the search tool absent (grep 127) all took the same silent
  # branch. The last two mean the assertion never ran, and a run this gate did not look
  # at must not be counted as a run that passed — the digests below would then be
  # compared across sessions nobody checked for crashes. Status read, not collapsed.
  st="$(gate_probe_file "path=vanilla-fallthrough" "$RAW")"
  case "$st" in
  0) echo "FAIL run $i: vanilla fall-through"; fail=1 ;;
  1) ;; # no match — clean, and we know it is clean because the search ran
  *) echo "FAIL run $i: vanilla fall-through check did not execute (grep exited $st on $RAW)"; fail=1 ;;
  esac
  # ANY script error fails the run — VM exceptions carry no [TBD] tag but their
  # stack traces implicate our code (measured: set.Remove-by-index crash was
  # invisible to a [TBD]-only grep for multiple gate runs).
  st="$(gate_probe_file "SCRIPT[[:space:]]*\(E\)|Virtual Machine Exception" "$RAW")"
  case "$st" in
  0)
    echo "FAIL run $i: script error lines:"; grep -E "SCRIPT[[:space:]]*\(E\)|Virtual Machine Exception" "$RAW" | head -5
    fail=1
    ;;
  1) ;;
  *) echo "FAIL run $i: script-error check did not execute (grep exited $st on $RAW)"; fail=1 ;;
  esac
  # Vanilla respawn/faction churn (measured 2026-07-24: 138 engine "has switched from
  # faction" lines at ~1 s, cycling US/USSR/FIA/CIV, because the vanilla spawn logic hunted
  # a spawn point that slot bodies had replaced). Our own deploy sets affiliation once, so a
  # healthy run emits 0-1 lines; 3 is headroom. The post-census check is the sharp one — a
  # switch after the audit line means the hunt loop is alive again.
  CHURN=$(grep -c "has switched from faction" "$RAW")
  if [ "${CHURN:-0}" -gt 3 ]; then echo "FAIL run $i: faction churn ($CHURN switch lines)"; fail=1; fi
  if sed -n "/\[TBD\]\[Audit\]/,\$p" "$RAW" | grep -q "has switched from faction"; then
    echo "FAIL run $i: faction switch AFTER census — churn loop alive"; fail=1
  fi
  # exactly one bind per player
  DUP=$(grep -oE "bound player [0-9]+" "$RAW" | sort | uniq -c | awk '$1 > 1' | head -3)
  if [ -n "$DUP" ]; then echo "FAIL run $i: duplicate binds:"; echo "$DUP"; fail=1; fi
  # every issued gear line resolves to a good outcome
  BAD_GEAR=$(grep -E "\[TBD\]\[Loadout\].*(FAILED|not worn)" "$RAW" | head -3)
  if [ -n "$BAD_GEAR" ]; then echo "FAIL run $i: gear failures:"; echo "$BAD_GEAR"; fail=1; fi
  # materialization model: characters in world == materialized slot bodies
  CENSUS=$(grep -oE "\[TBD\]\[Audit\] characters=[0-9]+ bodies=[0-9]+ players=[0-9]+" "$RAW" | tail -1)
  if [ -n "$CENSUS" ]; then
    C=$(echo "$CENSUS" | grep -oE "characters=[0-9]+" | cut -d= -f2)
    B=$(echo "$CENSUS" | grep -oE "bodies=[0-9]+" | cut -d= -f2)
    if [ "$C" != "$B" ]; then echo "FAIL run $i: census mismatch $CENSUS (stray/missing bodies?)"; fail=1; fi
  else
    echo "FAIL run $i: no census line"; fail=1
  fi
  # materialization happened
  if ! grep -qE "\[TBD\]\[Slots\] materialized [1-9]" "$RAW"; then echo "FAIL run $i: no materialization line"; fail=1; fi

  DIGESTS[$i]=$(sha256sum "$NORM" | cut -d' ' -f1)
  echo "run $i digest ${DIGESTS[$i]:0:12} ($(wc -l < "$NORM") lines)"
done

# cross-run digest identity
for i in $(seq 2 "$RUNS"); do
  if [ "${DIGESTS[$i]}" != "${DIGESTS[1]}" ]; then
    echo "FAIL: run $i digest differs from run 1 — first divergence:"
    diff "$OUT_DIR/run1.norm.log" "$OUT_DIR/run$i.norm.log" | head -15
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "DETERMINISM PASS: $RUNS/$RUNS identical (digest ${DIGESTS[1]:0:12})"
  [ "${TBD_DET_KEEP:-0}" = "1" ] || rm -rf "$OUT_DIR"
else
  echo "DETERMINISM FAIL — snapshots kept at $OUT_DIR"
fi
exit "$fail"
