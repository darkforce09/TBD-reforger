#!/usr/bin/env bash
# world-boot.sh — boot the REAL scenario headlessly and prove the game mode wired up.
#
# ── Why this exists ────────────────────────────────────────────────────────────────────────
# `compile.sh` proves the scripts compile. It cannot see prefab wiring at all: a component
# listed in `TBD_GameMode.et` whose class fails to resolve is dropped SILENTLY, compiles
# clean, and the only symptom is a feature that never runs. Three components (lobby,
# spectator, safestart) were added to that prefab across three separate slices with nothing
# proving any of them instantiates. This closes that hole for every future slice.
#
# ── The command-line landmine this script encodes ──────────────────────────────────────────
# Measured 2026-07-25, engine 1.7.0.54. Two combinations that look obviously right both fail:
#
#   -addons TBD_Framework -config server.json
#       -> DEFAULT (F): -config cannot be used together with addons!
#          ENGINE  (E): Unable to initialize the game
#       The two flags are MUTUALLY EXCLUSIVE. Hard fatal, no world.
#
#   -addons TBD_Framework -scenarioId "{...}Missions/TBD_Dev_POC.conf"   (no -config)
#       -> "Game successfully created", then nothing. The binary never starts hosting, so
#          the world never loads and no game-mode prefab is ever instantiated. This is the
#          harness that previously "proved" nothing either way — it looks successful.
#
# The combination that WORKS: `-addonsDir <dir>` plus `-config <json>`, with the addon listed
# in the config's `game.mods[]` keyed by the GUID from `addon.gproj` (NOT a Workshop id):
#
#   "mods": [ { "modId": "B2C3D4E5F6A78901", "name": "TBD_Framework" } ]
#
# ── What it asserts ────────────────────────────────────────────────────────────────────────
#   1. the world actually loaded          ("Starting new playthrough ... <scenarioId>")
#   2. the roll-call printed              (proves TBD_FrameworkManager itself instantiated)
#   3. no component reported MISSING      (proves every sibling on the prefab resolved)
#   4. no unexpected SCRIPT (E)           (allowlist below — boot runs without a backend)
#
# The roll-call is emitted by TBD_FrameworkManager.PrintComponentRollCall(). Its negative
# control is recorded in docs/mod/t181_event_mod_program.md: removing TBD_LobbyComponent from
# the prefab flips the line to `Lobby=MISSING` and this script to FAIL.
#
# Usage:
#   bash scripts/mod/world-boot.sh              # the gate
#   bash scripts/mod/world-boot.sh --selftest   # prove the verdict logic can FAIL
#   bash scripts/mod/world-boot.sh --keep-logs  # leave the run dir for inspection
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=../lib/hostrun.sh
source "$ROOT/scripts/lib/hostrun.sh"

MOD_SRC="$ROOT/apps/mod/tbd-framework"
SERVER_DIR="$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server"
SERVER_BIN="$SERVER_DIR/ArmaReforgerServer"
DEV_CONFIG="$ROOT/scripts/mod/tbd-dev-server.config.json"
MAX_WAIT="${TBD_WORLDBOOT_TIMEOUT:-240}"

KEEP_LOGS=0
SELFTEST=0
for arg in "$@"; do
  case "$arg" in
    --keep-logs) KEEP_LOGS=1 ;;
    --selftest)  SELFTEST=1 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

# SCRIPT (E) lines that are CORRECT on a bare boot: no backend is running and no mission is
# configured, so the loader is supposed to refuse. Anything else is a real runtime error.
# Kept as a grep -E alternation; widen only with a comment saying why the error is expected.
EXPECTED_ERRORS='missionId not configured'

# ── verdict logic, factored out so --selftest can exercise it against a synthetic log ──────
# Prints findings and returns non-zero on any failure.
assess_log() {
  local log="$1" scenario="$2" rc=0

  if ! grep -q "Starting new playthrough.*$scenario" "$log"; then
    echo "  FAIL  world never loaded (no 'Starting new playthrough' for $scenario)"
    rc=1
  else
    echo "  ok    world loaded"
  fi

  local rollcall
  rollcall="$(grep -oE '\[TBD\] roll-call:[^"]{0,200}' "$log" | head -1)"
  if [ -z "$rollcall" ]; then
    echo "  FAIL  no roll-call line — TBD_FrameworkManager did not instantiate"
    rc=1
  elif printf '%s' "$rollcall" | grep -q "MISSING"; then
    echo "  FAIL  component(s) declared on TBD_GameMode.et did not instantiate:"
    echo "        $rollcall"
    rc=1
  else
    echo "  ok    roll-call clean: ${rollcall#*roll-call: }"
  fi

  # Attribution matters here. Booting the real Eden world runs a lot of vanilla content, and
  # vanilla emits its own script errors that the mod neither causes nor can fix (measured:
  # `'SCR_BaseResupplySupportStationComponent' needs a entity catalog manager!`). Failing on
  # those would make the gate cry wolf until someone silences it wholesale — so the verdict
  # is scoped to errors TBD OWNS (a `[TBD]` tag or a Scripts/Game/TBD/ path), and vanilla
  # noise is reported with a count so a genuine change in it is still visible to a human.
  local errors mine theirs
  errors="$(grep -E 'SCRIPT +\(E\)' "$log" | grep -vE "$EXPECTED_ERRORS" || true)"
  mine="$(printf '%s\n' "$errors" | grep -E '\[TBD\]|Scripts/Game/TBD/' || true)"
  theirs="$(printf '%s\n' "$errors" | grep -vE '\[TBD\]|Scripts/Game/TBD/' | grep -E 'SCRIPT' || true)"

  if [ -n "$mine" ]; then
    echo "  FAIL  TBD script error(s) at boot:"
    printf '%s\n' "$mine" | head -8 | sed 's/^/        /'
    rc=1
  else
    echo "  ok    no TBD script errors"
  fi

  if [ -n "$theirs" ]; then
    local n
    n="$(printf '%s\n' "$theirs" | wc -l | tr -d ' ')"
    echo "  note  $n vanilla script error(s) (not TBD-owned, not failing):"
    printf '%s\n' "$theirs" | sed -E 's/.*SCRIPT +\(E\): //' | sort -u | head -4 | sed 's/^/        /'
  fi

  return "$rc"
}

# ── --selftest: prove the checker can actually FAIL ────────────────────────────────────────
# A gate that has never been seen to fail is not a gate. This feeds assess_log a synthetic
# log with a MISSING component and requires a non-zero verdict.
if [ "$SELFTEST" -eq 1 ]; then
  echo "==> world-boot selftest (verdict logic must reject a bad log)"
  t="$(mktemp -d "${TMPDIR:-/tmp}/tbd-wb-selftest.XXXXXX")"
  trap 'rm -rf "$t"' EXIT

  cat >"$t/good.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : [TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok
SCRIPT    (E): [TBD] missionId not configured — cannot load mission.
EOF
  cat >"$t/bad-missing.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT    (E): [TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=MISSING
EOF
  cat >"$t/bad-noworld.log" <<'EOF'
ENGINE       : Game successfully created.
EOF
  cat >"$t/bad-scripterr.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : [TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok
SCRIPT    (E): @"Scripts/Game/TBD/Boom.c,12": null pointer to instance
EOF

  # Vanilla noise must NOT fail the gate — this pins the attribution rule so nobody later
  # "fixes" it into a blanket error check that then gets silenced wholesale.
  cat >"$t/good-vanilla-noise.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : [TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok
SCRIPT    (E): 'SCR_BaseResupplySupportStationComponent' needs a entity catalog manager!
EOF

  SCEN='{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'
  st=0
  for good in good good-vanilla-noise; do
    echo "-- $good (must PASS)"
    if assess_log "$t/$good.log" "$SCEN" >/dev/null 2>&1; then echo "   PASS"; else echo "   FAIL: rejected $good"; st=1; fi
  done
  for bad in bad-missing bad-noworld bad-scripterr; do
    echo "-- $bad (must FAIL)"
    if assess_log "$t/$bad.log" "$SCEN" >/dev/null 2>&1; then
      echo "   FAIL: accepted $bad"; st=1
    else
      echo "   PASS (correctly rejected)"
    fi
  done
  [ "$st" -eq 0 ] && echo "SELFTEST OK" || echo "SELFTEST FAILED"
  exit "$st"
fi

# ── the real boot ──────────────────────────────────────────────────────────────────────────
require_host
[ -x "$SERVER_BIN" ] || { echo "ERROR: server binary not found at $SERVER_BIN" >&2; exit 1; }
[ -f "$DEV_CONFIG" ] || { echo "ERROR: dev config not found at $DEV_CONFIG" >&2; exit 1; }

ADDON_GUID="$(grep -oE '^\s*GUID\s+"[0-9A-Fa-f]+"' "$MOD_SRC/addon.gproj" | grep -oE '[0-9A-Fa-f]{8,}')"
[ -n "$ADDON_GUID" ] || { echo "ERROR: could not read GUID from $MOD_SRC/addon.gproj" >&2; exit 1; }
SCENARIO="$(grep -oE '"scenarioId"[^,]*' "$DEV_CONFIG" | grep -oE '\{[^}]+\}[^"]*')"
[ -n "$SCENARIO" ] || { echo "ERROR: could not read scenarioId from $DEV_CONFIG" >&2; exit 1; }

RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/tbd-worldboot.XXXXXX")"
mkdir -p "$RUN_DIR/addons" "$RUN_DIR/profile"
ln -sfn "$MOD_SRC" "$RUN_DIR/addons/tbd-framework"

# Same kill discipline as compile.sh: the launcher runs under `setsid`, so the recorded PID
# is a PROCESS GROUP LEADER and we signal the whole group. Never widen this to a name match —
# a broad `pkill -f ArmaReforgerServer` would take out the operator's own dev server.
PIDFILE="$RUN_DIR/server.pid"
kill_run() {
  local pgid
  pgid="$(cat "$PIDFILE" 2>/dev/null)" || return 0
  [ -n "$pgid" ] || return 0
  hostrun kill -TERM -- "-$pgid" >/dev/null 2>&1 || true
  for _ in $(seq 1 20); do
    hostrun kill -0 -- "-$pgid" >/dev/null 2>&1 || return 0
    sleep 0.25
  done
  hostrun kill -9 -- "-$pgid" >/dev/null 2>&1 || true
}
cleanup() {
  kill_run
  if [ "$KEEP_LOGS" -eq 1 ]; then
    echo "run dir kept: $RUN_DIR"
  else
    rm -rf "$RUN_DIR"
  fi
}
trap cleanup EXIT

# The config the engine actually gets: the committed dev config with the local addon injected
# into game.mods[]. Generated rather than committed so the GUID can never drift from the gproj.
python3 - "$DEV_CONFIG" "$RUN_DIR/server.json" "$ADDON_GUID" <<'PY'
import json, sys
src, dst, guid = sys.argv[1], sys.argv[2], sys.argv[3]
cfg = json.load(open(src))
cfg.setdefault("game", {})["mods"] = [{"modId": guid, "name": "TBD_Framework"}]
json.dump(cfg, open(dst, "w"), indent=2)
PY

echo "==> booting world (addon $ADDON_GUID, scenario $SCENARIO)"
hostrun env -C "$SERVER_DIR" setsid sh -c '
  echo $$ > "$1/server.pid"
  exec timeout '"$MAX_WAIT"' ./ArmaReforgerServer \
    -addonsDir "$1/addons" -config "$1/server.json" -profile "$1/profile" -maxFPS 15
' _ "$RUN_DIR" >/dev/null 2>&1 &
SRV_WAIT=$!

# Poll for the roll-call (the last thing we need) or a fatal, rather than always burning the
# full timeout. The roll-call fires one frame after the game mode's OnPostInit.
LOG=""
for _ in $(seq 1 "$((MAX_WAIT * 2))"); do
  LOG="$(ls -1d "$RUN_DIR"/profile/logs/logs_* 2>/dev/null | tail -1)/console.log"
  if [ -f "$LOG" ]; then
    grep -q '\[TBD\] roll-call' "$LOG" && break
    grep -qE '\(F\):|Unable to initialize the game' "$LOG" && break
  fi
  sleep 0.5
done

kill_run
# NOTE: do not infer failure from $SRV_WAIT — under setsid the local launcher returns early.
for _ in $(seq 1 10); do kill -0 "$SRV_WAIT" 2>/dev/null || break; sleep 0.2; done
kill "$SRV_WAIT" 2>/dev/null || true

if [ ! -f "$LOG" ]; then
  echo "ERROR: no console.log produced under $RUN_DIR/profile/logs" >&2
  exit 1
fi

# A fatal is worth surfacing verbatim — this is where the -config/-addons landmine shows up.
if grep -qE '\(F\):' "$LOG"; then
  echo "  FATAL from engine:"
  grep -E '\(F\):' "$LOG" | head -4 | sed 's/^/        /'
fi

if assess_log "$LOG" "$SCENARIO"; then
  echo "WORLD BOOT: PASS"
  exit 0
fi
echo "WORLD BOOT: FAIL"
[ "$KEEP_LOGS" -eq 1 ] || echo "  (re-run with --keep-logs to inspect the full console.log)"
exit 1
