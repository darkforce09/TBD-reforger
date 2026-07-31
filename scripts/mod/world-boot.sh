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
# ── EXIT CODES (identical contract to compile.sh — the two gates run back to back) ─────────
#   0  the world booted and every assertion above held
#   1  CODE: an assertion failed, or the API refused to produce a compiled document
#      (`api_doc_fail`). Something under test is broken.
#   2  usage — a bad argument, or --compiled together with --mission
#   3  ENVIRONMENT: this machine is not set up to run the gate (`env_fail` / `api_env_fail`).
#      The world was never booted, so a 3 says NOTHING about the mod. Do not read it as a code
#      failure. T-186 established this split for the --compiled API lane only; T-209 extended it
#      to the preconditions below, which were exiting 1 — i.e. reporting "no game installed" in
#      the same breath as "your component did not instantiate".
#   130/143  died on SIGINT/SIGTERM (128+signo), distinct from this script's own 1/2/3
#
# Usage:
#   bash scripts/mod/world-boot.sh                    # the gate
#   bash scripts/mod/world-boot.sh --selftest         # prove the verdict logic can FAIL
#   bash scripts/mod/world-boot.sh --keep-logs        # leave the run dir for inspection
#   bash scripts/mod/world-boot.sh --mission=<name>   # boot a hand-written golden
#   bash scripts/mod/world-boot.sh --compiled         # boot an API-COMPILED document (T-186)
#   bash scripts/mod/world-boot.sh --compiled=<uuid>  # ...of an EXISTING mission row
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=../lib/hostrun.sh
source "$ROOT/scripts/lib/hostrun.sh"

MOD_SRC="$ROOT/apps/mod/tbd-framework"
SERVER_DIR="$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server"
SERVER_BIN="$SERVER_DIR/ArmaReforgerServer"
DEV_CONFIG="$ROOT/scripts/mod/tbd-dev-server.config.json"
MAX_WAIT="${TBD_WORLDBOOT_TIMEOUT:-240}"

GOLDENS="$ROOT/packages/tbd-schema/golden-missions"
WARN_BASELINE="$ROOT/.world-boot-warning-baseline"

# The live dev API. Overridable so this can be pointed at staging without editing the script.
API_BASE="${TBD_API_BASE:-http://127.0.0.1:8080}"

KEEP_LOGS=0
SELFTEST=0
MISSION=""
COMPILED=0
COMPILED_UUID=""
# Baseline row this run ratchets against — see the RATCHET note in assess_log. Declared here so
# it exists before assess_log is ever defined, including on the --selftest path.
WARN_KEY=""
for arg in "$@"; do
  case "$arg" in
    --keep-logs)  KEEP_LOGS=1 ;;
    --selftest)   SELFTEST=1 ;;
    --mission=*)  MISSION="${arg#--mission=}" ;;
    --mission)    echo "use --mission=<file|name>" >&2; exit 2 ;;
    --compiled)   COMPILED=1 ;;
    --compiled=*) COMPILED=1; COMPILED_UUID="${arg#--compiled=}" ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

# Mutually exclusive on purpose: both write the same profile file, so accepting both would
# silently boot whichever ran last and report it under the other one's name.
if [ "$COMPILED" -eq 1 ] && [ -n "$MISSION" ]; then
  echo "ERROR: --compiled and --mission are mutually exclusive" >&2; exit 2
fi

# Resolve a bare golden name ("bridgehead-at-levie") to its file.
if [ -n "$MISSION" ] && [ ! -f "$MISSION" ]; then
  if [ -f "$GOLDENS/$MISSION.json" ]; then
    MISSION="$GOLDENS/$MISSION.json"
  elif [ -f "$GOLDENS/$MISSION" ]; then
    MISSION="$GOLDENS/$MISSION"
  else
    echo "ERROR: no such mission '$MISSION' (looked in $GOLDENS)" >&2; exit 2
  fi
fi

# Errors that are CORRECT on a bare boot: no backend is running and no mission is configured, so
# the loader is supposed to refuse. Widen only with a comment saying why the error is expected.
# `MissionList: backend not configured` is correct on a --mission boot: the mission comes from the
# local file fallback and there is deliberately no backend URL, so the browser has nothing to list.
# (It is logged at ERROR level for a legal state — a truth-in-logging nit filed under T-181.30.)
EXPECTED_ERRORS='missionId not configured|MissionList: backend not configured'

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

  # ── mission-seeded assertions ────────────────────────────────────────────────────────────
  if [ -n "${MISSION_ID:-}" ]; then
    local verdict errs warns budget
    verdict="$(grep -oE 'mission result=[A-Z]+ errors=[0-9]+ warnings=[0-9]+' "$log" | tail -1)"
    if [ -z "$verdict" ]; then
      echo "  FAIL  mission '$MISSION_ID' never reached the validator (no result line)"
      return 1
    fi
    errs="$(printf '%s' "$verdict"  | sed -E 's/.*errors=([0-9]+).*/\1/')"
    warns="$(printf '%s' "$verdict" | sed -E 's/.*warnings=([0-9]+).*/\1/')"

    if printf '%s' "$verdict" | grep -q 'result=PASS' && [ "$errs" = "0" ]; then
      echo "  ok    mission validated: $verdict"
    else
      echo "  FAIL  mission did not validate: $verdict"
      grep -oE '\[TBD\]\[Validate\][^"]{0,120}' "$log" | grep -iE 'error' | head -6 | sed 's/^/        /'
      rc=1
    fi

    # Warning RATCHET, same idea as compile.sh's vanilla baseline: a rise is a regression and
    # fails; a fall is progress and asks you to tighten the file. A budget you never tighten
    # decays into a rubber stamp, so the drop case is deliberately noisy.
    #
    # Keyed on WARN_KEY, not MISSION_ID, for one reason: a `--compiled` boot derives its
    # `meta.id` from a freshly-minted mission UUID, so it is DIFFERENT on every run and could
    # never match a baseline row — the ratchet would print "no baseline" forever and this lane
    # would ship with no warning budget at all. WARN_KEY pins that lane to one stable row.
    budget="$(grep -E "^$WARN_KEY[[:space:]]" "$WARN_BASELINE" 2>/dev/null | awk '{print $2}')"
    if [ -z "$budget" ]; then
      echo "  note  no warning baseline for $WARN_KEY (observed $warns) — add: '$WARN_KEY $warns' to $(basename "$WARN_BASELINE")"
    elif [ "$warns" -gt "$budget" ]; then
      echo "  FAIL  validator warnings rose: $warns > baseline $budget for $WARN_KEY"
      rc=1
    elif [ "$warns" -lt "$budget" ]; then
      echo "  note  warnings IMPROVED ($warns < baseline $budget) — tighten $(basename "$WARN_BASELINE") to '$WARN_KEY $warns'"
    else
      echo "  ok    validator warnings at baseline ($warns)"
    fi

    # T-609: name WHICH warnings fired, not just how many. A bare count over budget sent every
    # reader of this gate into --keep-logs archaeology for a fact the log already held — and a
    # ratchet that shows only a number is how five T-250 warnings sat unexamined for five waves.
    # Printed whenever any warning fired (not only on a rise): the at-baseline list is what a
    # tightening decision is made from, and the subjects are the evidence either way.
    if [ "${warns:-0}" -gt 0 ]; then
      grep -oE '\[TBD\]\[Validate\] WARNING .*' "$log" | sed 's/^\[TBD\]\[Validate\] WARNING /        warn  /' | head -10
      n_shown="$(grep -cE '\[TBD\]\[Validate\] WARNING ' "$log" || true)"
      if [ "${n_shown:-0}" -gt 10 ]; then
        echo "        … and $((n_shown - 10)) more (see console.log)"
      fi
    fi
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
# The general-purpose sibling of `api_env_fail` (which is defined further down, on the --compiled
# lane, and so cannot be reached from here). Same exit 3, same reason: an environment failure
# dressed as a code failure sends the next agent auditing a mod that is fine.
env_fail() {
  echo
  echo "WORLD BOOT: ENV FAIL — $1"
  echo "  This is the HARNESS's environment. The world was never booted, so this says NOTHING"
  echo "  about the mod — do not read it as a code failure."
  if [ -n "${2:-}" ]; then echo "  $2"; fi
  exit 3
}

# `require_host` was called BARE here until T-209. This script runs under `set -uo pipefail` with
# no `-e`, so its 127 was discarded: with no host bridge it printed a diagnosis and then carried
# on to fail on the next line with a different, wrong one.
require_host || env_fail "no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine" \
  "See scripts/lib/hostrun.sh: the container has no C toolchain and an older glibc, so the game binary cannot run in here at all."
[ -x "$SERVER_BIN" ] || env_fail "server binary not found at $SERVER_BIN" \
  "Install it from Steam (appid 1890870):  steam steam://install/1890870"
[ -f "$DEV_CONFIG" ] || env_fail "dev config not found at $DEV_CONFIG" \
  "The checkout does not look like this repo — verify the working tree before blaming the mod."

ADDON_GUID="$(grep -oE '^\s*GUID\s+"[0-9A-Fa-f]+"' "$MOD_SRC/addon.gproj" | grep -oE '[0-9A-Fa-f]{8,}')"
[ -n "$ADDON_GUID" ] || { echo "ERROR: could not read GUID from $MOD_SRC/addon.gproj" >&2; exit 1; }
SCENARIO="$(grep -oE '"scenarioId"[^,]*' "$DEV_CONFIG" | grep -oE '\{[^}]+\}[^"]*')"
[ -n "$SCENARIO" ] || { echo "ERROR: could not read scenarioId from $DEV_CONFIG" >&2; exit 1; }

RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/tbd-worldboot.XXXXXX")"
mkdir -p "$RUN_DIR/addons" "$RUN_DIR/profile"
ln -sfn "$MOD_SRC" "$RUN_DIR/addons/tbd-framework"

# ── --mission: seed a real mission so the document path actually RUNS ───────────────────────
# Without this the gate proves only that the game mode wires up: the loader refuses ("missionId
# not configured"), the stage machine never leaves LOADING, and the validator, zone registry,
# slot materialisation and marker service are ALL un-gated. The wave-5 verifier found two live
# MAJOR bugs sitting in exactly that blind spot, both visible in the first ~40 s of a
# mission-seeded boot.
#
# No backend is needed: TBD_MissionLoader falls back to a local file at
# `$profile:missions/<missionId>.json` (TBD_MissionLoader.LoadFromProfileFile — cite the NAME; the
# `:508` that used to be here had drifted to :561 by T-181.26).
#
# That fallback is also the mod's remaining UNVALIDATED input. T-181.31 made `GET /compiled`
# validate the exact bytes it serves, but LoadFromProfileFile applies NO json-schema validation at
# all — only TBD_MissionValidator, which is strictly more permissive than the schema. So a
# hand-staged or `--mission=`-seeded document can carry shapes the schema forbids (a blank
# `meta.name`, a blank `slot.groupCallsign`, a `slot.faction` that breaks its own pattern), and
# every consumer downstream has to treat empty and malformed as reachable states.
#
# LANDMINE: `$profile:` resolves to `<-profile-arg>/profile/`, NOT `<-profile-arg>/`. Seeding one
# level up loads nothing, silently — measured by the wave-5 verifier after two dead boots.
MISSION_ID=""

# Same kill discipline as compile.sh: the launcher runs under `setsid`, so the recorded PID
# is a PROCESS GROUP LEADER and we signal the whole group. Never widen this to a name match —
# a broad `pkill -f ArmaReforgerServer` would take out the operator's own dev server.
#
# Installed BEFORE the mission seeding below (it used to sit after) because `--compiled` creates a
# real row in the operator's mission library. If the fetch fails halfway — the POST lands, the
# `/compiled` GET 500s — the trap is what deletes it; without that, every failed run would leave
# another orphan draft in the library.
PIDFILE="$RUN_DIR/server.pid"

# THE FIXTURE'S TITLE, and the only handle cleanup needs. Interpolated into the seed body below
# so the two can never drift: a stale copy here would silently disarm the sweep, and the only
# symptom would be orphan drafts piling up in the operator's library with nothing reporting it.
FIXTURE_TITLE="T-186 compiled-boot fixture"

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

# Delete every mission row carrying the fixture title.
#
# Sweeping on the TITLE rather than on a uuid this process recorded is what makes the cleanup
# total, and it is not a stylistic preference — a uuid guard cannot cover three real cases:
#   * `create_mission` (handlers/missions.rs) COMMITS its transaction before it serialises the
#     response, so a `-m 30` timeout or a dropped socket after the commit leaves a live row whose
#     id this script never learned. The POST's own failure path then exits with nothing to delete.
#   * a 201 whose body carries no `id` — same shape, row committed, id unknown here.
#   * `kill -9`, which skips traps entirely and cannot be made to clean up after itself at all.
# The title sweep covers all three, and because it is not scoped to this run it SELF-HEALS rows
# left behind by earlier ones.
#
# It cannot touch an operator's mission. The title is harness-generated, and the DELETE is
# authorised as the dev-login user, which owns nothing else (`can_edit` in handlers/missions.rs is
# author-or-admin). `--compiled=<uuid>` never mints that token, so on that lane this is a no-op —
# an existing row the operator names is never a candidate for deletion.
#
# Soft delete, same as the SPA's own: `deleted_at` is stamped and `/ingest/missions` filters on
# `deleted_at IS NULL`, so "swept" means invisible to the next run, not physically gone.
sweep_fixture_missions() {
  [ -n "${SVC_TOKEN:-}" ] && [ -n "${DEV_ACCESS_TOKEN:-}" ] || return 0
  local listing id
  listing="$RUN_DIR/sweep.json"
  curl -sS -o "$listing" -m 10 -H "X-Service-Token: $SVC_TOKEN" \
    "$API_BASE/api/v1/ingest/missions" >/dev/null 2>&1 || return 0
  # `{"missions":[{"id","name",...}],"count"}` — the same envelope the reachability probe hits.
  for id in $(python3 -c "import json,sys
d = json.load(open(sys.argv[1]))
print('\n'.join(m['id'] for m in d.get('missions', []) if m.get('name') == sys.argv[2]))" \
    "$listing" "$FIXTURE_TITLE" 2>/dev/null); do
    curl -sS -o /dev/null -m 10 -X DELETE "$API_BASE/api/v1/missions/$id" \
      -H "Authorization: Bearer $DEV_ACCESS_TOKEN" >/dev/null 2>&1 || true
  done
}

# Idempotent: the signal traps below call this and then `exit`, which re-enters it through the
# EXIT trap. Without the guard that second pass would re-run the whole sweep (another list + N
# deletes) on the way out of a Ctrl-C.
CLEANED=0
cleanup() {
  [ "$CLEANED" -eq 0 ] || return 0
  CLEANED=1
  kill_run
  sweep_fixture_missions   # must precede the rm — it writes its listing into RUN_DIR
  if [ "$KEEP_LOGS" -eq 1 ]; then
    echo "run dir kept: $RUN_DIR"
  else
    rm -rf "$RUN_DIR"
  fi
}
trap cleanup EXIT
# EXIT alone leaves the interrupt path to bash's wait-and-cooperative-exit heuristic, and that
# heuristic is CONDITIONAL: a non-interactive shell dies of a Ctrl-C only when the foreground
# child it happened to be waiting on was itself killed by that same SIGINT.
#
# Measured at a real pty on bash 5.2.15 (a synthetic `kill -INT -- -PGID` from another shell is
# not the same delivery path and will mislead you): with EXIT only, one Ctrl-C in the poll loop
# below DID stop the script and its EXIT trap DID run. So this is not a leak in the common case.
# It holds by luck rather than by construction, though — any foreground command that absorbs
# SIGINT and returns 0 leaves this shell looping with the row already seeded — and the process
# died OF the signal, so the caller saw "terminated by signal 2" rather than a status it could
# branch on.
#
# An explicit trap removes both. 128+signo is the shell's own convention for "died on this
# signal", so 130/143 still read as a signal death while staying distinguishable from this
# script's own 1 / 2 / 3. cleanup is idempotent, so the `exit` re-entering it via EXIT is a no-op.
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM

# ── --compiled: feed an API-COMPILED document to the real parser (T-186) ────────────────────
# Everything above boots HAND-WRITTEN goldens, and the goldens are strictly RICHER than anything
# `crates/map-engine-core/src/mission/flatten.rs` can emit (they carry radioPlan, briefings,
# objectives, multiple factions — none of which the compiler produces). So a green
# `--mission=<golden>` says nothing about the documents the website actually serves.
#
# Three lanes existed and never met: flatten.rs unit tests assert in Rust only; the API tests hold
# `/compiled` to mission.schema.json but never parse it with Enfusion; this script boots real
# missions but only goldens. Every website↔mod contract drift was therefore found in production —
# T-181.46 is the proof (the compiler emitted `endOn:["faction_eliminated"]` for a mission where
# only one faction held slots, and TBD_MissionValidator hard-rejected the whole document; the
# schema was perfectly happy with it, so the API tests were green the entire time).
#
# This lane closes that: seed through the real API, fetch the real bytes from `GET /compiled`, and
# hand THOSE to the real Enfusion parser. The fixture is deliberately the THINNEST document the
# compiler can produce — one faction holding slots, which is what a freshly-authored editor
# mission looks like and is exactly the shape T-181.46 died on.
#
# FAILURE DISCIPLINE (the whole point of the helpers below): an env failure reported as a
# code failure sends the next agent chasing a bug that does not exist. `api_env_fail` = the stack
# is not up / the token is wrong / the row is not there; `api_doc_fail` = the API read the
# document and REFUSED it, which is a compiler or contract defect. curl's own exit status is one
# discriminator — a transport failure never yields an HTTP status — but it is not the only one,
# because plenty of ENVIRONMENT failures do come back as a clean HTTP status. See `api_http_fail`.
api_env_fail() {
  echo
  echo "COMPILED BOOT: ENV FAIL — $1"
  echo "  This is the HARNESS's environment. The mod was never started, so this says NOTHING"
  echo "  about the mod or the compiler — do not read it as a code failure."
  echo "  ${2:-Bring the stack up and re-run:  make db-up && make api   (API expected at $API_BASE)}"
  exit 3
}
api_doc_fail() {
  echo
  echo "COMPILED BOOT: FAIL — $1"
  echo "  The API would not produce a compiled document. That is a COMPILER/CONTRACT defect,"
  echo "  not an environment one — re-running will not fix it. Check the API log: a 500 here is"
  echo "  'compiled mission failed schema validation' (validated_compiled_body in"
  echo "  apps/website/api/src/handlers/missions.rs), a 409 is 'no placed slots'."
  exit 1
}
# Route a non-success HTTP status to whichever of the two above is TRUE for it. Sending everything
# non-2xx to `api_doc_fail` — which asserts outright that "re-running will not fix it" — is worse
# than merely imprecise: an unattended fix agent handed that verdict for a stopped Postgres will
# spend its entire retry budget auditing a compiler that is working. Two environment failures
# reach here with a perfectly clean HTTP status:
#   * 5xx — the API could not serve the request at all. A down or unmigrated database surfaces as
#     an sqlx error, which the ApiError type renders as a 500.
#   * 404 — nothing at that id. `--compiled=<bad-uuid>` lands here, and no amount of staring at
#     flatten.rs will produce the row.
# Everything else stays on the compiler verdict, because 400 (payload rejected), 409 ('no placed
# slots') and 422 all mean the API looked at the document and said no — exactly what this lane
# exists to catch. A 500 out of `/compiled` is the ambiguous one: it is usually the schema
# validation `api_doc_fail` describes, but it is indistinguishable from a DB fault at this layer,
# and the API log (which `api_env_fail` does not send you to) is where that gets settled.
api_http_fail() {
  local code="$1" what="$2" doc_msg="$3"
  case "$code" in
    404) api_env_fail "$what -> HTTP 404 — nothing at that id/route on $API_BASE" \
                      "Check the mission id you passed and that the API is the one you think it is." ;;
    5??) api_env_fail "$what -> HTTP $code — the API could not serve the request. A stopped or unmigrated Postgres surfaces here as a 500; the API log says which." \
                      "Check the API log first, then:  make db-up && make api   (API expected at $API_BASE)" ;;
    *)   api_doc_fail "$doc_msg" ;;
  esac
}

# The API's SERVICE_TOKEN. `apps/website/api/.env` is gitignored, so it does NOT exist in a slice
# worktree — fall back to the MAIN checkout's copy. `--git-common-dir` is shared by every worktree
# and points at the main repo's .git, so its parent is the main working tree (the same resolution
# scripts/platform/wave.sh uses for CARGO_TARGET_DIR, and for the same reason).
resolve_service_token() {
  if [ -n "${TBD_SERVICE_TOKEN:-}" ]; then printf '%s' "$TBD_SERVICE_TOKEN"; return 0; fi
  local common main_root f tok
  common="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"
  main_root="$(dirname "${common:-$ROOT/.git}")"
  for f in "$ROOT/apps/website/api/.env" "$main_root/apps/website/api/.env"; do
    [ -f "$f" ] || continue
    tok="$(sed -n 's/^SERVICE_TOKEN=//p' "$f" | head -1 | tr -d '\r' | sed 's/^["'"'"']//;s/["'"'"']$//')"
    [ -n "$tok" ] && { printf '%s' "$tok"; return 0; }
  done
  return 1
}

if [ "$COMPILED" -eq 1 ]; then
  echo "==> seeding a compiled mission via $API_BASE"
  SVC_TOKEN="$(resolve_service_token)" || api_env_fail \
    "no SERVICE_TOKEN — set TBD_SERVICE_TOKEN, or add it to apps/website/api/.env"

  # Reachability and the token in ONE probe, before anything is created. `/ingest/missions` is the
  # cheapest service-token route there is.
  probe_code="$(curl -sS -o /dev/null -w '%{http_code}' -m 10 \
    -H "X-Service-Token: $SVC_TOKEN" "$API_BASE/api/v1/ingest/missions" 2>"$RUN_DIR/curl.err")"
  probe_rc=$?
  [ "$probe_rc" -eq 0 ] || api_env_fail \
    "API unreachable at $API_BASE (curl exit $probe_rc: $(tr '\n' ' ' < "$RUN_DIR/curl.err"))"
  [ "$probe_code" = "401" ] && api_env_fail \
    "service token rejected (GET /api/v1/ingest/missions -> 401) — SERVICE_TOKEN does not match the running API"
  [ "$probe_code" = "200" ] || api_env_fail \
    "service-token probe GET /api/v1/ingest/missions -> HTTP $probe_code (expected 200)"

  if [ -n "$COMPILED_UUID" ]; then
    echo "    using existing mission $COMPILED_UUID"
  else
    # POST /missions is mission_maker+, so this needs a USER session, not the service token.
    # dev-login mints exactly that with no Discord round-trip — development-only, which is also
    # why a 404 here is an env failure (APP_ENV is not `development`), not a code one.
    DEV_ACCESS_TOKEN="$(curl -sS -o /dev/null -D - -m 10 \
      "$API_BASE/api/v1/auth/dev-login?role=mission_maker" 2>/dev/null \
      | tr -d '\r' | sed -n 's/.*[#&]access_token=\([^&]*\).*/\1/p' | head -1)"
    [ -n "$DEV_ACCESS_TOKEN" ] || api_env_fail \
      "dev-login returned no access_token — is the API running with APP_ENV=development?"

    # THE FIXTURE. Editor-graph shape (crates/map-engine-core/src/mission/flatten.rs `EditorPayload`),
    # deliberately minimal and deliberately SINGLE-FACTION:
    #   * one faction holding slots  -> the flatten pads a stub opponent and narrows `endOn` to
    #     `["time_limit"]`. This is the T-181.46 shape.
    #   * slot 3 carries `position.z` -> the document compiles as schemaVersion "1.2" (optional `y`),
    #     so the mod's 1.1-vs-1.2 branch is exercised, not just the default. 136.0 is not arbitrary:
    #     TBD_SpawnManager warns "jsonY deviates >2 m from surfaceY — stale DEM or mis-authored
    #     slot?" and Everon's surface at (4890, 7780) is 135.844 (measured). An arbitrary elevation
    #     made the harness emit that warning on EVERY run — a fixture artefact that reads exactly
    #     like a real DEM defect, which is the sort of thing that costs an agent an afternoon.
    #   * slot 2 carries a SlotLoadoutV2 -> the compiled `slot.loadout {gear,cargo}` block is
    #     exercised. The ResourceNames are copied from golden-missions/slot-loadout-coverage.json,
    #     which boots clean today, so an equip error here means the COMPILE broke, not the fixture.
    # LANDMINE: this heredoc is UNQUOTED so `$FIXTURE_TITLE` interpolates — the title cleanup
    # sweeps on and the title this POSTs are then provably the same string. The cost is that a
    # literal `$`, backtick or backslash added to the fixture below would be expanded by bash;
    # there are none today (Enfusion ResourceNames are `{GUID}Path/…`, and `{}` is inert in a
    # heredoc). Escape any you add, or the seed body silently changes shape.
    cat >"$RUN_DIR/seed.json" <<JSON
{
  "title": "$FIXTURE_TITLE",
  "terrain": "everon",
  "game_mode": "pvp",
  "weather": "clear",
  "time_of_day": "05:30",
  "max_players": 8,
  "briefing": "Generated by scripts/mod/world-boot.sh --compiled. Safe to delete.",
  "payload": {
    "schemaVersion": 1,
    "editor": {
      "factions": [{ "key": "blufor", "name": "US Army", "squadIds": ["sq_alpha"] }],
      "squads": [{ "id": "sq_alpha", "callsign": "Alpha", "name": "Alpha", "slotIds": ["sl_sl", "sl_ar", "sl_rfl"] }],
      "slots": [
        {
          "id": "sl_sl", "index": 0, "role": "SL",
          "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et",
          "position": { "x": 4870.0, "y": 7760.0, "z": 0.0, "rotation": 45.0 }
        },
        {
          "id": "sl_ar", "index": 1, "role": "AR",
          "assetId": "{5B1996C05B1E51A4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_AR.et",
          "position": { "x": 4880.0, "y": 7770.0, "z": 0.0, "rotation": 90.0 },
          "loadout": {
            "wear": {
              "jacket": "{293F577C298061E3}Prefabs/Characters/Uniforms/Jacket_US_BDU_02.et",
              "armoredVest": "{477A190AF2A17B8A}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_MG.et",
              "headCover": "{B74A4FF0DD8BB116}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01.et",
              "pants": "{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et",
              "boots": "{DAAFD15478BDE1C3}Prefabs/Characters/Footwear/CombatBoots_US_01.et"
            },
            "weapons": [
              {
                "slotIndex": 0, "slotType": "primary",
                "weapon": "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
                "magazine": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et"
              }
            ],
            "cargo": [
              {
                "container": "vest",
                "item": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "qty": 6
              }
            ]
          }
        },
        {
          "id": "sl_rfl", "index": 2, "role": "RFL",
          "assetId": "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
          "position": { "x": 4890.0, "y": 7780.0, "z": 136.0, "rotation": 315.0 }
        }
      ]
    }
  }
}
JSON
    seed_code="$(curl -sS -o "$RUN_DIR/seed-resp.json" -w '%{http_code}' -m 30 \
      -X POST "$API_BASE/api/v1/missions" \
      -H "Authorization: Bearer $DEV_ACCESS_TOKEN" -H 'Content-Type: application/json' \
      --data-binary @"$RUN_DIR/seed.json" 2>"$RUN_DIR/curl.err")"
    seed_rc=$?
    [ "$seed_rc" -eq 0 ] || api_env_fail \
      "POST /api/v1/missions transport failure (curl exit $seed_rc: $(tr '\n' ' ' < "$RUN_DIR/curl.err"))"
    if [ "$seed_code" != "201" ]; then
      echo "  POST /api/v1/missions -> HTTP $seed_code"
      head -c 600 "$RUN_DIR/seed-resp.json"; echo
      api_http_fail "$seed_code" "POST /api/v1/missions" \
        "the API rejected the editor payload this harness seeds (HTTP $seed_code)"
    fi
    COMPILED_UUID="$(python3 -c "import json,sys;print(json.load(open(sys.argv[1])).get('id',''))" "$RUN_DIR/seed-resp.json")"
    # Both exits above this line, and this one, can fire with the row ALREADY COMMITTED — which is
    # why cleanup sweeps on the title and not on an id recorded here. Do not "optimise" that back
    # into a uuid guard.
    [ -n "$COMPILED_UUID" ] || api_doc_fail "POST /api/v1/missions 201 but returned no mission id"
    echo "    seeded mission $COMPILED_UUID"
  fi

  # THE POINT OF THE WHOLE SLICE: these are the exact bytes the game server gets in production.
  comp_code="$(curl -sS -o "$RUN_DIR/compiled.json" -w '%{http_code}' -m 60 \
    -H "X-Service-Token: $SVC_TOKEN" \
    "$API_BASE/api/v1/missions/$COMPILED_UUID/compiled" 2>"$RUN_DIR/curl.err")"
  comp_rc=$?
  [ "$comp_rc" -eq 0 ] || api_env_fail \
    "GET /compiled transport failure (curl exit $comp_rc: $(tr '\n' ' ' < "$RUN_DIR/curl.err"))"
  if [ "$comp_code" != "200" ]; then
    echo "  GET /api/v1/missions/$COMPILED_UUID/compiled -> HTTP $comp_code"
    head -c 1200 "$RUN_DIR/compiled.json"; echo
    api_http_fail "$comp_code" "GET /api/v1/missions/$COMPILED_UUID/compiled" \
      "GET /compiled -> HTTP $comp_code (expected 200)"
  fi

  # Hand the fetched bytes to the SAME staging path `--mission=` uses, byte-for-byte. Copying
  # rather than re-serialising matters: the mod must parse what the API served, not a
  # round-tripped equivalent.
  MISSION="$RUN_DIR/compiled.json"
  WARN_KEY="compiled"
  echo "    fetched $(wc -c <"$MISSION" | tr -d ' ') bytes of compiled document"
fi

if [ -n "$MISSION" ]; then
  MISSION_ID="$(python3 -c "import json,sys;print(json.load(open(sys.argv[1])).get('meta',{}).get('id',''))" "$MISSION")"
  [ -n "$MISSION_ID" ] || { echo "ERROR: $MISSION has no meta.id" >&2; exit 1; }
  mkdir -p "$RUN_DIR/profile/profile/missions"
  cp "$MISSION" "$RUN_DIR/profile/profile/missions/$MISSION_ID.json"
  printf '{"backendUrl":"","serverToken":"","missionId":"%s","eventId":""}\n' \
    "$MISSION_ID" > "$RUN_DIR/profile/profile/TBD_BackendConfig.json"
  # The registry's primary path is `$TBD_Framework:Data/registry.json`, and that alias does NOT
  # resolve for a loose addon — `Data/*.json` is a non-script resource, so it is gated by the same
  # stale `resourceDatabase.rdb` as the menu presets. Without the profile fallback every slot fails
  # with "kit resolve failed", which is a real TBD error and correctly fails this gate.
  cp "$MOD_SRC/Data/registry.json" "$RUN_DIR/profile/profile/TBD_Registry.json"
  # Goldens ratchet under their own stable `meta.id`; `--compiled` already pinned WARN_KEY above
  # because its meta.id is UUID-derived and changes every run.
  [ -n "$WARN_KEY" ] || WARN_KEY="$MISSION_ID"
fi

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
    if [ -n "${MISSION_ID:-}" ]; then
      # Mission mode waits for the validator verdict, which lands well after the roll-call.
      grep -q 'mission result=' "$LOG" && break
    else
      grep -q '\[TBD\] roll-call' "$LOG" && break
    fi
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
  # ENV, not code. The engine writes console.log before it loads a single script, so "no log at
  # all" cannot be caused by anything in the mod — the binary never got far enough to read it.
  # An unwritable profile dir, a wiped Steam depot or a killed process all land here.
  env_fail "no console.log produced under $RUN_DIR/profile/logs — the engine never started writing" \
    "Check that $SERVER_BIN runs at all and that ${TMPDIR:-/tmp} is writable."
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
