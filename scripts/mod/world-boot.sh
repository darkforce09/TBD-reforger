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
#   1. the world actually loaded            ("Starting new playthrough ... <scenarioId>")
#   2. no unresolvable class / VM exception (`WORLD (E): Unknown class`, engine-reported)
#   3. the roll-call printed                (proves TBD_FrameworkManager itself instantiated)
#   4. no roll-call entry reported MISSING  (proves the LISTED siblings resolved)
#   5. no TBD-owned and no UNRECOGNISED script error (fail-closed; see VANILLA_BENIGN)
#
# ── What it does NOT assert (do not over-read a green result) ──────────────────────────────
# * **Only checks 2 and 4 together cover the prefab.** The roll-call is a hand-maintained list of
#   names, so on its own it is blind to a component nobody added to it — the wave-4 verifier
#   proved this by adding `TBD_ThisComponentDoesNotExist` to the prefab and getting a PASS.
#   Check 2 is what actually generalises. `SCR_EditableEntityComponent` is on the prefab and
#   deliberately not in the roll-call, which is fine precisely because check 2 exists.
# * **No runtime behaviour.** The boot has no backend and no configured mission, so the loader
#   correctly refuses, the stage machine never leaves LOADING, no slot body is materialised,
#   safestart never arms and no player ever joins. A green world-boot says the game mode WIRES
#   UP. It says nothing about whether any of it WORKS. That needs T-181.16/.25 (dedicated
#   server + real clients).
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

# Errors that are CORRECT on a bare boot: no backend is running and no mission is configured, so
# the loader is supposed to refuse. Widen only with a comment saying why the error is expected.
EXPECTED_ERRORS='missionId not configured'

# Engine diagnostics that mean STRUCTURAL breakage, failed regardless of who "owns" the message.
# `WORLD (E): Unknown class` is the important one: it is how the engine reports a component on a
# prefab whose class does not resolve, it names the offending class itself, and — unlike the
# roll-call — it needs no per-component maintenance and covers EVERY prefab in the mod, not just
# TBD_GameMode.et. The wave-4 verifier proved the roll-call alone is insufficient by adding
# `TBD_ThisComponentDoesNotExist` to the prefab and getting a PASS; this line is what catches it.
# `Virtual Machine Exception` is a script crash that does not always carry a `SCRIPT (E)` tag.
HARD_FAIL='WORLD +\(E\): Unknown class|Virtual Machine Exception|Unable to find component class|Cannot find component'

# Vanilla script errors known to be emitted by the stock Eden world, which the mod neither causes
# nor can fix. This is an ALLOWLIST and the check is FAIL-CLOSED: an error matching nothing here
# and nothing in TBD is reported as unrecognised and FAILS. That inversion is deliberate — the
# previous rule classified by message text ("not obviously TBD, therefore vanilla, therefore fine")
# and the verifier passed six different genuine TBD failures through it, including one that
# differed only in the case of `Scripts/Game/`. Add a pattern here only with a reason.
VANILLA_BENIGN='needs a entity catalog manager'

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

  # Structural breakage first — these fail whoever "owns" the text, and catch the component-does-
  # not-resolve case by name rather than by a list this script has to keep in sync with a prefab.
  local hard
  hard="$(grep -E "$HARD_FAIL" "$log" || true)"
  if [ -n "$hard" ]; then
    echo "  FAIL  engine reported structural breakage:"
    printf '%s\n' "$hard" | sed -E 's/^[0-9:. ]*//' | sort -u | head -6 | sed 's/^/        /'
    rc=1
  else
    echo "  ok    no unresolvable classes / VM exceptions"
  fi

  # `Print(someLocalVariable)` in Enfusion emits the DECLARATION, not just the value
  # (`string line = '[TBD] roll-call: …'`), so this must not anchor to start-of-field, and the
  # trailing quote is stripped below rather than assumed absent. Measured, not guessed.
  local rollcall
  rollcall="$(grep -oE "\[TBD\] roll-call:.*" "$log" | head -1)"
  rollcall="${rollcall%\'}"
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

  # Script errors, triaged fail-closed. `mine` is matched case-INSENSITIVELY and on the bare
  # `TBD` token as well as `[TBD]`/path, because the engine emits both `@"Scripts/Game/…"` and
  # `@"scripts/Game/…"` in one run, and messages like `Instance of class TBD_SpawnManager is
  # null` carry neither a tag nor a path. Anything left over is UNRECOGNISED and fails.
  local errors mine benign unknown
  errors="$(grep -E 'SCRIPT +\(E\)' "$log" | grep -vE "$EXPECTED_ERRORS" || true)"
  errors="$(printf '%s' "$errors" | grep -E 'SCRIPT' || true)"
  mine="$(printf '%s\n' "$errors" | grep -iE '\[tbd\]|tbd_|/tbd/' || true)"
  benign="$(printf '%s\n' "$errors" | grep -ivE '\[tbd\]|tbd_|/tbd/' | grep -E "$VANILLA_BENIGN" || true)"
  unknown="$(printf '%s\n' "$errors" | grep -ivE '\[tbd\]|tbd_|/tbd/' | grep -vE "$VANILLA_BENIGN" | grep -E 'SCRIPT' || true)"

  if [ -n "$mine" ]; then
    echo "  FAIL  TBD script error(s) at boot:"
    printf '%s\n' "$mine" | head -8 | sed 's/^/        /'
    rc=1
  else
    echo "  ok    no TBD script errors"
  fi

  if [ -n "$unknown" ]; then
    echo "  FAIL  unrecognised script error(s) — neither TBD-owned nor a known-benign vanilla"
    echo "        pattern. If genuinely vanilla and harmless, add it to VANILLA_BENIGN with a"
    echo "        reason; do NOT widen the TBD match to make it disappear."
    printf '%s\n' "$unknown" | sed -E 's/.*SCRIPT +\(E\): //' | sort -u | head -6 | sed 's/^/        /'
    rc=1
  fi

  if [ -n "$benign" ]; then
    local n
    n="$(printf '%s\n' "$benign" | wc -l | tr -d ' ')"
    echo "  note  $n known-benign vanilla script error(s), not failing:"
    printf '%s\n' "$benign" | sed -E 's/.*SCRIPT +\(E\): //' | sort -u | head -4 | sed 's/^/        /'
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

  # NOTE the roll-call shape: `Print(localVar)` in Enfusion emits the DECLARATION and quotes the
  # value. These fixtures use the REAL measured shape — the earlier ones used an idealised
  # `SCRIPT : [TBD] roll-call: …` that never occurs, so the selftest was not exercising reality.
  cat >"$t/good.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): [TBD] missionId not configured — cannot load mission.
EOF
  cat >"$t/bad-missing.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT    (E): string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=MISSING'
EOF
  cat >"$t/bad-noworld.log" <<'EOF'
ENGINE       : Game successfully created.
EOF
  cat >"$t/bad-scripterr.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): @"Scripts/Game/TBD/Boom.c,12": null pointer to instance
EOF

  # ── the wave-4 verifier's escapes ────────────────────────────────────────────────────────
  # Every one of these PASSED the original message-text attribution rule. They are pinned here
  # so the fail-closed triage can never silently regress to "not obviously TBD, therefore fine".
  #
  # bad-unknown-class is the big one: a component on the prefab whose class does not exist. The
  # roll-call CANNOT catch it (it only checks names it was told about), so this is caught by the
  # engine's own `WORLD (E): Unknown class` diagnostic instead.
  cat >"$t/bad-unknown-class.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
WORLD     (E): Unknown class 'TBD_ThisComponentDoesNotExist' at offset 530(0x212)
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
EOF
  cat >"$t/bad-vm-exception.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT       : Virtual Machine Exception - Null pointer to instance in TBD_SafestartManager::Restore
EOF
  cat >"$t/bad-lowercase-path.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): @"scripts/game/tbd/Gamemode/TBD_SpawnManager.c,1400": null pointer to instance
EOF
  cat >"$t/bad-untagged-tbd.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): Instance of class TBD_SpawnManager is null
EOF
  cat >"$t/bad-unrecognised.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): Resource file worlds/SomeOther.ent not found
EOF

  # Known-benign vanilla noise must NOT fail the gate — this pins the one allowlisted pattern so
  # nobody later "fixes" it into a blanket error check that then gets silenced wholesale.
  cat >"$t/good-vanilla-noise.log" <<'EOF'
DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): 'SCR_BaseResupplySupportStationComponent' needs a entity catalog manager!
EOF

  SCEN='{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'
  st=0
  for good in good good-vanilla-noise; do
    echo "-- $good (must PASS)"
    if assess_log "$t/$good.log" "$SCEN" >/dev/null 2>&1; then echo "   PASS"; else echo "   FAIL: rejected $good"; st=1; fi
  done
  for bad in bad-missing bad-noworld bad-scripterr bad-unknown-class bad-vm-exception \
             bad-lowercase-path bad-untagged-tbd bad-unrecognised; do
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
#
# Ports are moved off the committed dev values (2001 / 17777). The gate never needs a REACHABLE
# server — only a loaded world — so binding the real dev ports buys nothing and costs a collision
# with the operator's own dev server or with a second concurrent gate run. Derived from the PID so
# two parallel runs differ. (The wave-4 verifier hit exactly this and had to hand-repoint to 2051.)
BIND_PORT=$(( 21000 + ($$ % 4000) ))
A2S_PORT=$(( 26000 + ($$ % 4000) ))
python3 - "$DEV_CONFIG" "$RUN_DIR/server.json" "$ADDON_GUID" "$BIND_PORT" "$A2S_PORT" <<'PY'
import json, sys
src, dst, guid = sys.argv[1], sys.argv[2], sys.argv[3]
bind_port, a2s_port = int(sys.argv[4]), int(sys.argv[5])
cfg = json.load(open(src))
cfg["bindPort"] = bind_port
cfg["publicPort"] = bind_port
if isinstance(cfg.get("a2s"), dict):
    cfg["a2s"]["port"] = a2s_port
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
# full timeout. Measured: the roll-call fires on the first callqueue tick after the world enters
# the online game state, ~1.6 s after the game mode entity is initialised — NOT literally one
# frame after OnPostInit, though one frame would be sufficient for its purpose.
LOG=""
for _ in $(seq 1 "$((MAX_WAIT * 2))"); do
  LOG="$(ls -1d "$RUN_DIR"/profile/logs/logs_* 2>/dev/null | tail -1)/console.log"
  if [ -f "$LOG" ]; then
    grep -q '\[TBD\] roll-call' "$LOG" && break
    grep -qE '\(F\):|Unable to initialize the game' "$LOG" && break
  fi
  sleep 0.5
done

# Settle before killing. Without this the capture window ends the instant the roll-call appears,
# which is non-deterministic with respect to everything the world is still doing: two runs of the
# identical tree differed by two vanilla errors purely on timing. That matters in both directions
# — a late TBD error would be missed, and the fail-closed unknown-error check would be flaky.
# The window is still BOUNDED, so this is not proof that no error ever occurs after it; it just
# makes the same tree give the same verdict.
sleep "${TBD_WORLDBOOT_SETTLE:-4}"

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
