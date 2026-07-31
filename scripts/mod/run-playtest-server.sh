#!/usr/bin/env bash
# run-playtest-server.sh — start a JOINABLE, mod-loaded, admin-capable dedicated server.
#
# ── Why this exists (T-604) ────────────────────────────────────────────────────────────────
# Nothing in this repo started a server two people could join with the LOCAL mod loaded.
# `run-dev-server.sh` was 27 lines that ran two preflight checks and ended — it never launched
# anything. `deploy-staging.sh` builds two ExecStarts and each one breaks a different half:
#
#   :1155  -addonsDir + -addons + -server   loads the local mod, registers NO backend room
#   :1153  -config (no -addonsDir)          registers a room, cannot resolve the local mod
#
# ── What is actually true (measured 2026-07-31, engine 1.7.0.54, this file's boot) ──────────
# `-addonsDir <dir>` **plus** `-config <json>` does BOTH at once. Verbatim, one boot:
#
#   ENGINE : FileSystem: Adding relative directory '<checkout>/apps/mod/tbd-framework'
#            to filesystem under name TBD_Framework
#   ENGINE : Loaded addons:
#            gproj: '<addonsDir>/tbd-framework/addon.gproj' guid: 'B2C3D4E5F6A78901'
#   NETWORK: Starting RPL server, listening on address 0.0.0.0:2001, fastValidation=true
#   BACKEND: Server registered with address: 192.168.0.117:2001
#   BACKEND: Direct Join Code: 0207990185
#
# So the room DOES register with the local addon loaded. `docs/mod/STAGING-SERVER.md` said this
# was impossible without a Workshop publish; that was measured on `-addons`, never on `-addonsDir`.
#
# ── THE TRAP THIS SCRIPT EXISTS TO CLOSE ───────────────────────────────────────────────────
# `tbd-framework` IS published to the Workshop, unlisted, under the SAME id as the local gproj
# GUID (`B2C3D4E5F6A78901`), at a stale **version 1.0.1**. So `-config` on its own does not fail
# loudly — the engine quietly downloads that June build and runs it:
#
#   BACKEND: Addon Download started B2C3D4E5F6A78901 - TBD Framework
#   BACKEND: Downloading B2C3D4E5F6A78901 version 1.0.1
#   ENGINE : FileSystem: Adding package '<profile>/addons/TBDFramework_B2C3D4E5F6A78901/'
#            (pak count: 1) to filesystem under name TBD_Framework
#
# A `-config`-only boot therefore looks completely healthy — it registers a room, it reaches
# LOBBY — while running months-old script. Measured difference on the same mission: 7 `[TBD]`
# log lines in the flat `[TBD] ...` format of June, versus 109 lines in the current
# `[TBD][Subsystem] ...` format. That is this codebase's signature defect wearing the engine's
# clothes, so `assert_local_addon_won` below is a HARD GATE, not a warning: if the packed
# profile copy wins, this script kills the server and exits non-zero.
#
# ── EXIT CODES (same contract as world-boot.sh / compile.sh) ───────────────────────────────
#   0  server booted, local addon won, backend room registered — join details printed
#   1  CODE/CONFIG: the server died, refused the config, or loaded the WRONG addon copy
#   2  usage
#   3  ENVIRONMENT: this machine cannot run the gate at all (no host bridge, no game installed)
#
# A `1` can ALSO mean "this script could not confirm the server died" — see the STRAY SERVER
# block. That block names the process group and the exact command to run; if you see it, the
# run dir is NOT clean and the next boot will fail on the port until you deal with it.
#
# Usage:
#   bash scripts/mod/run-playtest-server.sh --mission-id=<id> [options]
#   bash scripts/mod/run-playtest-server.sh --mission-id=<id> --admin=<identityId> --dry-run
#   bash scripts/mod/run-playtest-server.sh --selftest
#
# Options:
#   --mission-id=<id>     mission the mod loads (TBD_BackendConfig.json missionId)   [required]
#   --mission-file=<p>    stage <p> as the on-disk fallback for that id (no API needed)
#   --event-id=<id>       roster event id
#   --backend-url=<url>   default http://127.0.0.1:8080
#   --token=<tok>         SERVICE_TOKEN; default read from apps/website/api/.env
#   --admin=<id>          identityId (UUID) or 17-digit SteamID; repeatable
#   --name=<s>            server browser name
#   --scenario=<id>       scenarioId override (default: from tbd-dev-server.config.json)
#   --port=<n>            game port, default 2001
#   --a2s-port=<n>        A2S port, default 17777 (MUST differ from --port)
#   --max-players=<n>     default 8
#   --run-dir=<dir>       staging root, default $HOME/tbd-playtest
#   --timeout=<sec>       stop the server after <sec> (default: run until Ctrl-C)
#   --dry-run             render + validate everything, print the command line, boot nothing
#   --selftest            prove kill_run + the run lock actually work; boots no game server
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=../lib/hostrun.sh
source "$ROOT/scripts/lib/hostrun.sh"

MOD_SRC="$ROOT/apps/mod/tbd-framework"
SERVER_DIR="$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server"
SERVER_BIN="$SERVER_DIR/ArmaReforgerServer"
DEV_CONFIG="$ROOT/scripts/mod/tbd-dev-server.config.json"

MISSION_ID=""
MISSION_FILE=""
EVENT_ID=""
BACKEND_URL="http://127.0.0.1:8080"
TOKEN=""
SERVER_NAME=""
SCENARIO=""
GAME_PORT=2001
A2S_PORT=17777
MAX_PLAYERS=8
RUN_DIR="$HOME/tbd-playtest"
RUN_TIMEOUT=""
DRY_RUN=0
SELFTEST=0
ADMINS=()

usage_fail() { echo "ERROR: $1" >&2; echo "Usage: bash scripts/mod/run-playtest-server.sh --mission-id=<id> [--admin=<id>] [--dry-run]" >&2; exit 2; }

# ENVIRONMENT, not code — the world was never booted, so a 3 says NOTHING about the mod.
# Same split world-boot.sh:355 established; keep the two readable side by side.
env_fail() {
  echo "" >&2
  echo "ENVIRONMENT: $1" >&2
  if [ -n "${2:-}" ]; then echo "  $2" >&2; fi
  exit 3
}

for arg in "$@"; do
  case "$arg" in
    --mission-id=*)   MISSION_ID="${arg#*=}" ;;
    --mission-file=*) MISSION_FILE="${arg#*=}" ;;
    --event-id=*)     EVENT_ID="${arg#*=}" ;;
    --backend-url=*)  BACKEND_URL="${arg#*=}" ;;
    --token=*)        TOKEN="${arg#*=}" ;;
    --admin=*)        ADMINS+=("${arg#*=}") ;;
    --name=*)         SERVER_NAME="${arg#*=}" ;;
    --scenario=*)     SCENARIO="${arg#*=}" ;;
    --port=*)         GAME_PORT="${arg#*=}" ;;
    --a2s-port=*)     A2S_PORT="${arg#*=}" ;;
    --max-players=*)  MAX_PLAYERS="${arg#*=}" ;;
    --run-dir=*)      RUN_DIR="${arg#*=}" ;;
    --timeout=*)      RUN_TIMEOUT="${arg#*=}" ;;
    --dry-run)        DRY_RUN=1 ;;
    --selftest)       SELFTEST=1 ;;
    # Prints the Usage + Options comment block above verbatim. Bounded by `set -uo pipefail`'s
    # line so it cannot silently drift into code if the header grows.
    -h|--help)        sed -n '/^# Usage:/,/^set -uo pipefail/p' "$0" | grep -E '^#' | sed 's/^# \?//'; exit 0 ;;
    *)                usage_fail "unknown argument: $arg" ;;
  esac
done

# ═══ KILL DISCIPLINE, THE LIVENESS PROBE, AND THE RUN LOCK (T-608) ═════════════════════════
# Defined this early on purpose, ahead of every other check, for two reasons: `--selftest` has
# to be able to reach them without a mission id, and `assert_no_live_server` has to run BEFORE
# staging rewrites server.json underneath a server that is still running.
PIDFILE="$RUN_DIR/server.pid"
SRV_OUT="$RUN_DIR/server.out"
LOCKDIR="$RUN_DIR/.run.lock"

# ── THE PROBE ──────────────────────────────────────────────────────────────────────────────
# Everything below rests on this one function telling the truth, so it is written to make one
# specific lie impossible.
#
# WHAT IT REPLACED, AND WHY (measured 2026-07-31). The old aliveness check was:
#
#     hostrun kill -0 -- "-$pgid" >/dev/null 2>&1 || return 0     # "|| it's gone"
#
# Every probe is a SEPARATE host-bridge process, and a bridge that fails to start exits
# non-zero in exactly the same way `kill -0` does on a dead pid. The two are indistinguishable
# at the rc. So one bridge failure read as death: the escalation was skipped, `kill_run`
# returned success, and the script exited 1 announcing "the server never registered a backend
# room" while the engine was still alive and holding 2001/17777. The operator had to find and
# kill process group 3870163 by hand. That is this repo's signature defect — a tool reporting
# a result over an input it never actually examined — living inside the very script written to
# stop a dead server being reported as up.
#
# THE FIX IS A SENTINEL. The far side prints `TBDPROBE=alive|zombie|dead` itself. The answer is
# believed only when it demonstrably came back from a probe that RAN on the host. Anything
# else — no bridge, empty output, an error string, a partial read — is `unknown`, and `unknown`
# is NOT death and is never once treated as it.
#
# `zombie` is split out from `alive` because a reaped-but-unwaited group leader still answers
# `kill -0` while holding no sockets; folding that into "alive" would make death permanently
# unconfirmable and turn the STRAY warning into a false alarm. If the host has no `pgrep`,
# `seen` stays 0 and the answer degrades to the conservative `alive`, never to `dead`.
#
# Echoes exactly one of: alive | zombie | dead | unknown
probe_group() {
  local pgid="$1" out
  [ -n "$pgid" ] || { printf 'unknown'; return 0; }
  out="$(hostrun sh -c '
    p=$1
    if kill -0 -- "-$p" 2>/dev/null; then
      live=0; seen=0
      for q in $(pgrep -g "$p" 2>/dev/null); do
        seen=$((seen + 1))
        st=$(sed -n "s/^State:[[:space:]]*\([A-Z]\).*/\1/p" "/proc/$q/status" 2>/dev/null)
        [ "$st" = "Z" ] || live=$((live + 1))
      done
      if [ "$seen" -gt 0 ] && [ "$live" -eq 0 ]; then echo "TBDPROBE=zombie"; else echo "TBDPROBE=alive"; fi
    else
      echo "TBDPROBE=dead"
    fi
  ' _ "$pgid" 2>/dev/null)"
  case "$out" in
    *TBDPROBE=alive*)  printf 'alive'   ;;
    *TBDPROBE=zombie*) printf 'zombie'  ;;
    *TBDPROBE=dead*)   printf 'dead'    ;;
    *)                 printf 'unknown' ;;
  esac
}

# Set by kill_run when it could not CONFIRM death. Printed last, by print_stray_warning, so it
# is the final thing on screen rather than something scrolled off by a diagnosis dump.
STRAY_PGID=""

print_stray_warning() {
  [ -n "$STRAY_PGID" ] || return 0
  cat >&2 <<EOF

================================================================================
  STRAY SERVER — this script could NOT confirm the server died.

    process group: $STRAY_PGID
    pidfile:       $PIDFILE   (LEFT IN PLACE deliberately — it is the only
                   handle on that group, and a stale pidfile pointing at a live
                   process is worth more than no pidfile at all)

  Do this yourself and check the second command comes back empty:

      distrobox-host-exec kill -9 -- -$STRAY_PGID     # from inside this container
      kill -9 -- -$STRAY_PGID                          # from a host terminal
      pgrep -af '[A]rmaReforgerServer'

  Until that group is gone it still holds UDP $GAME_PORT / $A2S_PORT, and the next boot
  will die with 'NETWORK (E): Unable to start replication' — which looks like a
  different bug entirely. Once it IS gone, delete the pidfile:

      rm -f '$PIDFILE'
================================================================================
EOF
  return 1
}

# Stop the server and PROVE it stopped.
#   rc 0  the process group is confirmed gone (or there was never one to stop)
#   rc 1  could not confirm — STRAY_PGID is set and print_stray_warning has the details
#
# Deliberately NOT a name match: a broad `pkill -f ArmaReforgerServer` would also kill the
# operator's own dev server, and (measured) the bridge's own `sh -c` command line contains that
# string, so it kills the caller too. The recorded pid is a PROCESS GROUP LEADER — the launcher
# runs under `setsid` — and we signal the whole group, same discipline as world-boot.sh:423.
kill_run() {
  local pgid state waited
  pgid="$(cat "$PIDFILE" 2>/dev/null)"
  pgid="$(printf '%s' "${pgid:-}" | tr -d '[:space:]')"
  # No pidfile is not evidence of death, but it is also nothing we can act on: there is no
  # group id to signal and kill-by-name is off the table. Say so rather than implying success.
  if [ -z "$pgid" ]; then
    return 0
  fi

  state="$(probe_group "$pgid")"
  if [ "$state" = "dead" ] || [ "$state" = "zombie" ]; then
    rm -f "$PIDFILE"
    return 0
  fi

  # TERM first. The engine honours TERM at steady state (measured: `--timeout=30` produced
  # `Game destroyed` at T+31 s and a clean exit 0) but IGNORED it during world load — which is
  # exactly when this function fires on a failed boot. So the grace is a grace, not a promise,
  # and it is followed by KILL unconditionally.
  hostrun kill -TERM -- "-$pgid" >/dev/null 2>&1 || true
  waited=0
  while [ "$waited" -lt 40 ]; do          # 40 x 0.25 s = 10 s
    waited=$((waited + 1))
    sleep 0.25
    state="$(probe_group "$pgid")"
    if [ "$state" = "dead" ] || [ "$state" = "zombie" ]; then
      rm -f "$PIDFILE"
      return 0
    fi
  done

  # Still here, or still unanswerable. BOTH escalate. "I could not tell" must never take the
  # same branch as "I confirmed it is dead" — that equivalence is the whole defect.
  echo "    TERM did not settle process group $pgid after 10s (state: $state) — escalating to KILL" >&2
  hostrun kill -9 -- "-$pgid" >/dev/null 2>&1 || true
  waited=0
  while [ "$waited" -lt 20 ]; do          # 20 x 0.25 s = 5 s
    waited=$((waited + 1))
    sleep 0.25
    state="$(probe_group "$pgid")"
    if [ "$state" = "dead" ] || [ "$state" = "zombie" ]; then
      rm -f "$PIDFILE"
      return 0
    fi
  done

  # SIGKILL cannot be caught, so reaching here means either the signal never landed (the bridge
  # is down) or the process is wedged in the kernel. Either way we do NOT know it is dead, we do
  # NOT delete the pidfile, and we do NOT return success.
  STRAY_PGID="$pgid"
  return 1
}

# ── the run lock (T-608 / F5) ──────────────────────────────────────────────────────────────
# There was no lock and the run dir is fixed, so running the S3 "restart with --admin" command
# before Ctrl-C'ing the first server orphaned the running group: the second invocation's
# `rm -f "$PIDFILE"` destroyed the only handle, the first instance's kill_run then read no
# pidfile and reported "server exited" while its engine was still up, and the new boot died on
# the port. Two guards now, because they answer different questions:
#   claim_lock            — is another copy of THIS SCRIPT using this run dir?
#   assert_no_live_server — is a SERVER still running from a previous invocation of it?
# The second one matters on its own: a script that was killed leaves no lock but can very much
# leave a server.
LOCK_HELD=0

release_lock() {
  [ "$LOCK_HELD" -eq 1 ] || return 0
  rm -rf "$LOCKDIR" 2>/dev/null || true
  LOCK_HELD=0
}

claim_lock() {
  local owner
  mkdir -p "$RUN_DIR" 2>/dev/null || true
  if mkdir "$LOCKDIR" 2>/dev/null; then
    printf '%s\n' "$$" >"$LOCKDIR/owner" 2>/dev/null || true
    LOCK_HELD=1
    trap 'release_lock' EXIT
    return 0
  fi

  # Read the owner with a short retry. `mkdir` and the write of `owner` are two steps, so a
  # second copy starting in that window would see an empty file and wrongly call the lock stale
  # — which would defeat the entire guard at exactly the moment it is needed.
  local tries=0
  owner=""
  while [ "$tries" -lt 20 ]; do
    owner="$(cat "$LOCKDIR/owner" 2>/dev/null | tr -d '[:space:]')"
    [ -z "$owner" ] || break
    tries=$((tries + 1))
    sleep 0.1
  done
  # The lock owner is another instance of this script, in this pid namespace — a plain local
  # `kill -0` is the right question here and needs no bridge.
  if [ -n "$owner" ] && kill -0 "$owner" 2>/dev/null; then
    echo "" >&2
    echo "REFUSING: another run-playtest-server.sh (pid $owner) already owns $RUN_DIR." >&2
    echo "  Stop it first (Ctrl-C in its terminal) and let it print that it stopped." >&2
    echo "  Starting a second one here would rewrite server.json under the running server," >&2
    echo "  destroy its pidfile, and then die on port $GAME_PORT." >&2
    echo "  To run two servers at once, give this one its own dir and ports:" >&2
    echo "      --run-dir=$RUN_DIR-2 --port=2011 --a2s-port=17787" >&2
    exit 1
  fi

  echo "    note: taking over a stale lock (owner pid ${owner:-unknown} is gone)"
  printf '%s\n' "$$" >"$LOCKDIR/owner" 2>/dev/null || true
  LOCK_HELD=1
  trap 'release_lock' EXIT
  return 0
}

# Refuse to stage over a server that is still running. Fails CLOSED: "I cannot tell" refuses
# too, because the cost of being wrong is clobbering a live session's config.
assert_no_live_server() {
  local pgid state
  pgid="$(cat "$PIDFILE" 2>/dev/null)"
  pgid="$(printf '%s' "${pgid:-}" | tr -d '[:space:]')"
  [ -n "$pgid" ] || return 0

  state="$(probe_group "$pgid")"
  case "$state" in
    dead|zombie)
      rm -f "$PIDFILE"
      return 0
      ;;
    alive)
      echo "" >&2
      echo "REFUSING: a server from a previous run is STILL RUNNING (process group $pgid)." >&2
      echo "  $PIDFILE points at it and it is alive right now." >&2
      echo "" >&2
      echo "  Stop it first — Ctrl-C in its terminal if you still have it, otherwise:" >&2
      echo "      distrobox-host-exec kill -TERM -- -$pgid    # then check it is gone:" >&2
      echo "      distrobox-host-exec pgrep -af '[A]rmaReforgerServer'" >&2
      echo "" >&2
      echo "  This is deliberate. Booting anyway would rewrite server.json under it, replace" >&2
      echo "  the pidfile that is the only handle on it, and then fail on port $GAME_PORT with" >&2
      echo "  'Unable to start replication' — three problems instead of one." >&2
      exit 1
      ;;
    *)
      echo "" >&2
      echo "REFUSING: $PIDFILE names process group $pgid and this script could not reach the" >&2
      echo "  host bridge to find out whether it is still alive." >&2
      echo "  'I cannot tell' is not 'it is dead', so this refuses rather than guessing." >&2
      echo "  Check by hand, then delete the pidfile if the group really is gone:" >&2
      echo "      distrobox-host-exec pgrep -af '[A]rmaReforgerServer'" >&2
      echo "      rm -f '$PIDFILE'" >&2
      exit 1
      ;;
  esac
}

# ── --selftest: prove the kill path can FAIL, and cannot lie ───────────────────────────────
# Same principle as world-boot.sh:264 — a gate nobody has watched fail is not a gate. This one
# exists because T-608's defect was invisible on every passing run: kill_run only lied when the
# bridge flaked, which no green boot ever exercises. So the lie is reproduced here on purpose.
# Boots no game server; spawns disposable `sleep` groups on the host and kills them.
if [ "$SELFTEST" -eq 1 ]; then
  echo "==> run-playtest-server selftest (kill discipline must be unable to claim a false death)"
  require_host || env_fail "no host bridge — the selftest exercises the real bridge, so it needs one"

  ST_RC=0
  ST_TMP="$(mktemp -d "${TMPDIR:-/tmp}/tbd-rps-selftest.XXXXXX")"
  PIDFILE="$ST_TMP/server.pid"
  st_pass() { echo "  ok    $1"; }
  st_fail() { echo "  FAIL  $1"; ST_RC=1; }

  # Spawn a real setsid process group on the host and echo its pgid. $1 is shell code the
  # group leader runs; that is how the TERM-ignoring case is built.
  st_spawn() {
    local out
    out="$(hostrun sh -c '
      f=$(mktemp /tmp/tbd-rps-pg.XXXXXX)
      setsid sh -c "echo \$\$ > $f; $1" >/dev/null 2>&1 &
      n=0
      while [ ! -s "$f" ] && [ "$n" -lt 50 ]; do n=$((n+1)); sleep 0.1; done
      cat "$f"; rm -f "$f"
    ' _ "$1" 2>/dev/null)"
    printf '%s' "$out" | tr -d '[:space:]'
  }

  # S1 — THE REGRESSION. A live group plus a broken bridge must never be called dead.
  echo "  -- S1: live group + broken host bridge"
  ST_PG="$(st_spawn 'sleep 120')"
  if [ -z "$ST_PG" ]; then
    st_fail "S1 could not spawn a test group on the host"
  else
    printf '%s\n' "$ST_PG" >"$PIDFILE"
    # The override lives in a subshell, so the real hostrun is untouched afterwards. rc and
    # STRAY_PGID are read back out of the subshell's stdout because it cannot export them.
    ST_OUT="$( ( hostrun() { return 127; }; kill_run >/dev/null 2>&1; echo "rc=$? stray=$STRAY_PGID" ) )"
    case "$ST_OUT" in
      "rc=0"*) st_fail "S1 kill_run returned SUCCESS with the group alive — this is the T-608 defect" ;;
      *)       st_pass "S1 kill_run refused to claim success ($ST_OUT)" ;;
    esac
    case "$ST_OUT" in
      *"stray=$ST_PG"*) st_pass "S1 named the stray process group ($ST_PG) instead of exiting quietly" ;;
      *)                st_fail "S1 did not record the stray pgid: $ST_OUT" ;;
    esac
    [ -f "$PIDFILE" ] && st_pass "S1 kept the pidfile (it is the only handle on a live group)" \
                      || st_fail "S1 deleted the pidfile of a process it never confirmed dead"
    [ "$(probe_group "$ST_PG")" = "alive" ] && st_pass "S1 the group really was alive throughout" \
                      || st_fail "S1 test group died on its own — the case did not exercise anything"
    hostrun kill -9 -- "-$ST_PG" >/dev/null 2>&1 || true
  fi

  # S2 — TERM ignored, exactly as the engine ignores it during world load. Must escalate to
  # KILL, confirm the death, and only then drop the pidfile.
  echo "  -- S2: group that ignores SIGTERM (models the engine during world load)"
  STRAY_PGID=""
  ST_PG="$(st_spawn 'trap "" TERM; sleep 120')"
  if [ -z "$ST_PG" ]; then
    st_fail "S2 could not spawn a test group on the host"
  else
    printf '%s\n' "$ST_PG" >"$PIDFILE"
    kill_run >/dev/null 2>&1; ST_KRC=$?
    [ "$ST_KRC" -eq 0 ] && st_pass "S2 kill_run escalated past the ignored TERM and returned 0" \
                        || st_fail "S2 kill_run returned $ST_KRC against a killable group"
    [ "$(probe_group "$ST_PG")" = "dead" ] && st_pass "S2 the group is CONFIRMED gone, not assumed gone" \
                        || st_fail "S2 returned success while the group still answers"
    [ -f "$PIDFILE" ] && st_fail "S2 left a pidfile behind for a confirmed-dead group" \
                      || st_pass "S2 removed the pidfile only after confirming death"
    hostrun kill -9 -- "-$ST_PG" >/dev/null 2>&1 || true
  fi

  # S3 — the ordinary case still works, and no pidfile is not an error.
  echo "  -- S3: cooperative group, and the empty case"
  STRAY_PGID=""
  ST_PG="$(st_spawn 'sleep 120')"
  if [ -z "$ST_PG" ]; then
    st_fail "S3 could not spawn a test group on the host"
  else
    printf '%s\n' "$ST_PG" >"$PIDFILE"
    kill_run >/dev/null 2>&1; ST_KRC=$?
    [ "$ST_KRC" -eq 0 ] && [ "$(probe_group "$ST_PG")" = "dead" ] \
      && st_pass "S3 TERM path confirmed the death and returned 0" \
      || st_fail "S3 cooperative kill did not confirm (rc=$ST_KRC state=$(probe_group "$ST_PG"))"
    hostrun kill -9 -- "-$ST_PG" >/dev/null 2>&1 || true
  fi
  rm -f "$PIDFILE"
  kill_run >/dev/null 2>&1 && st_pass "S3 no pidfile is rc 0, not an invented failure" \
                           || st_fail "S3 no pidfile should be rc 0"

  # S4 — refuse-if-running. The pidfile names a live group; staging must not proceed.
  echo "  -- S4: assert_no_live_server refuses, and leaves the pidfile alone"
  ST_PG="$(st_spawn 'sleep 120')"
  if [ -z "$ST_PG" ]; then
    st_fail "S4 could not spawn a test group on the host"
  else
    printf '%s\n' "$ST_PG" >"$PIDFILE"
    # It exits on refusal, so run it in a subshell and read the exit status.
    ST_OUT="$( (assert_no_live_server) 2>&1 )"; ST_KRC=$?
    [ "$ST_KRC" -ne 0 ] && st_pass "S4 refused to stage over a live server (rc $ST_KRC)" \
                        || st_fail "S4 allowed staging over a live server"
    case "$ST_OUT" in
      *"STILL RUNNING"*"$ST_PG"*) st_pass "S4 named the running process group" ;;
      *)                          st_fail "S4 refusal did not name the group: $ST_OUT" ;;
    esac
    [ -f "$PIDFILE" ] && st_pass "S4 left the first run's pidfile untouched" \
                      || st_fail "S4 destroyed the first run's pidfile — the F5 orphan bug"
    hostrun kill -9 -- "-$ST_PG" >/dev/null 2>&1 || true
    sleep 0.5
    ST_OUT="$( (assert_no_live_server) 2>&1 )"; ST_KRC=$?
    [ "$ST_KRC" -eq 0 ] && st_pass "S4 allows staging once the group is confirmed dead" \
                        || st_fail "S4 still refuses after the group died: $ST_OUT"
    [ -f "$PIDFILE" ] && st_fail "S4 kept a pidfile for a confirmed-dead group" \
                      || st_pass "S4 cleared the pidfile only after confirming death"
  fi

  rm -rf "$ST_TMP"
  echo ""
  if [ "$ST_RC" -eq 0 ]; then echo "SELFTEST: PASS"; else echo "SELFTEST: FAIL"; fi
  exit "$ST_RC"
fi
# ═══ end kill discipline ═══════════════════════════════════════════════════════════════════

[ -n "$MISSION_ID" ] || usage_fail "--mission-id is required — it is what the mod loads"

# `a2sPort` and `bindPort` are separate UDP sockets. Equal ports make the engine log
# `NETWORK (E): Unable to start replication` and exit **status 0**, so nothing downstream
# notices (docs/mod/STAGING-SERVER.md). Refuse here instead of at boot.
[ "$GAME_PORT" != "$A2S_PORT" ] || usage_fail "--port and --a2s-port must differ (got $GAME_PORT for both); standard layout is 2001 game / 17777 A2S"

# ── admin ids: validate against the ENGINE's own schema, before boot ────────────────────────
# Both patterns copied verbatim out of the engine's rejection of a bad value (1.7.0.54):
#   BACKEND (E): RegEx Pattern: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
#   BACKEND (E): RegEx Pattern: "^[0-9]{17}$"
# A bad entry is a HARD FATAL at boot ("There are errors in server config!" -> "Unable to
# initialize the game"), and the engine reports it ~90 s in, after a full script compile. Failing
# here costs a millisecond and names the value.
for a in ${ADMINS+"${ADMINS[@]}"}; do
  if ! printf '%s' "$a" | grep -qE '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' \
  && ! printf '%s' "$a" | grep -qE '^[0-9]{17}$'; then
    echo "ERROR: --admin='$a' is neither an identityId nor a SteamID." >&2
    echo "  identityId: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx  (lowercase hex)" >&2
    echo "  SteamID:    17 digits" >&2
    echo "  The engine rejects anything else and refuses to start; this is its schema, not ours." >&2
    exit 2
  fi
done

require_host || env_fail "no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine" \
  "See scripts/lib/hostrun.sh: the container has an older glibc, so the game binary cannot run in here at all."
[ -x "$SERVER_BIN" ] || env_fail "server binary not found at $SERVER_BIN" \
  "Install it from Steam (appid 1890870):  steam steam://install/1890870"
[ -f "$DEV_CONFIG" ] || env_fail "dev config not found at $DEV_CONFIG" \
  "The checkout does not look like this repo — verify the working tree before blaming the mod."
[ -d "$MOD_SRC" ] || env_fail "mod source not found at $MOD_SRC"

# GUID read out of addon.gproj, never hardcoded — world-boot.sh:376 does the same, for the same
# reason: a literal here would drift from the gproj silently and the mod would stop resolving.
ADDON_GUID="$(grep -oE '^[[:space:]]*GUID[[:space:]]+"[0-9A-Fa-f]+"' "$MOD_SRC/addon.gproj" | grep -oE '[0-9A-Fa-f]{8,}')"
[ -n "$ADDON_GUID" ] || { echo "ERROR: could not read GUID from $MOD_SRC/addon.gproj" >&2; exit 1; }

if [ -z "$SCENARIO" ]; then
  SCENARIO="$(grep -oE '"scenarioId"[^,]*' "$DEV_CONFIG" | grep -oE '\{[^}]+\}[^"]*')"
fi
[ -n "$SCENARIO" ] || { echo "ERROR: could not read scenarioId from $DEV_CONFIG" >&2; exit 1; }

# The LAN address the room advertises. Resolved on the HOST: inside a container the default
# route can belong to the podman bridge, and a room registered on 10.88.x.x is unreachable from
# the friend's machine while looking perfectly healthy in the log.
LAN_IP="$(hostrun sh -c "ip route get 1.1.1.1 2>/dev/null | awk '{print \$7; exit}'" 2>/dev/null | tr -d '[:space:]')"
[ -n "$LAN_IP" ] || env_fail "could not determine this machine's LAN IP" \
  "Pass it by hand: edit publicAddress in $RUN_DIR/server.json after a --dry-run."

[ -n "$SERVER_NAME" ] || SERVER_NAME="TBD Playtest ($MISSION_ID)"

echo "==> staging $RUN_DIR"
# BEFORE a single byte of this run dir is rewritten. Both guards can exit; both say what to do.
claim_lock
assert_no_live_server
mkdir -p "$RUN_DIR/addons"

# ── profile ────────────────────────────────────────────────────────────────────────────────
# `$profile:` resolves to <-profile-arg>/profile/, NOT <-profile-arg>/ (world-boot.sh:383).
# setup-server-profile.sh already knows that; do not seed one level up.
bash "$ROOT/scripts/mod/setup-server-profile.sh" "$RUN_DIR/profile" >/dev/null || \
  env_fail "setup-server-profile.sh failed" "Run it directly to see why: bash scripts/mod/setup-server-profile.sh $RUN_DIR/profile"

BACKEND_CFG="$RUN_DIR/profile/profile/TBD_BackendConfig.json"
[ -f "$BACKEND_CFG" ] || env_fail "setup-server-profile.sh did not produce $BACKEND_CFG"

# Token: explicit flag wins; otherwise setup-server-profile.sh already substituted the one from
# apps/website/api/.env and we leave its work alone.
python3 - "$BACKEND_CFG" "$MISSION_ID" "$EVENT_ID" "$BACKEND_URL" "$TOKEN" <<'PY'
import json, sys
p, mid, eid, url, tok = sys.argv[1:6]
d = json.load(open(p))
d["missionId"] = mid
d["eventId"] = eid
d["backendUrl"] = url
if tok:
    d["serverToken"] = tok
json.dump(d, open(p, "w"), indent=2)
PY
[ $? -eq 0 ] || { echo "ERROR: could not patch $BACKEND_CFG" >&2; exit 1; }

if [ -n "$MISSION_FILE" ]; then
  [ -f "$MISSION_FILE" ] || usage_fail "--mission-file=$MISSION_FILE does not exist"
  # TBD_MissionLoader.LoadFromProfileFile reads $profile:missions/<missionId>.json, so the file
  # on disk must be named for the ID, not for the golden it came from (setup-server-profile.sh
  # carries the same note). Copy rather than re-serialise: the mod must parse these exact bytes.
  mkdir -p "$RUN_DIR/profile/profile/missions"
  cp "$MISSION_FILE" "$RUN_DIR/profile/profile/missions/$MISSION_ID.json"
  echo "    staged $(wc -c <"$MISSION_FILE" | tr -d ' ') bytes as the on-disk fallback for $MISSION_ID"
fi

# ── addon staging dir ──────────────────────────────────────────────────────────────────────
# A symlink to the live checkout, exactly like deploy-staging.sh:1100. This is the copy that
# must win at load time; assert_local_addon_won below proves it did.
ln -sfn "$MOD_SRC" "$RUN_DIR/addons/tbd-framework"

# ── server config ──────────────────────────────────────────────────────────────────────────
SERVER_JSON="$RUN_DIR/server.json"
ADMINS_JSON="$(printf '%s\n' ${ADMINS+"${ADMINS[@]}"} | python3 -c 'import json,sys; print(json.dumps([l for l in sys.stdin.read().split("\n") if l]))')"
python3 - "$DEV_CONFIG" "$SERVER_JSON" "$LAN_IP" "$GAME_PORT" "$A2S_PORT" "$MAX_PLAYERS" \
         "$ADDON_GUID" "$SCENARIO" "$SERVER_NAME" "$ADMINS_JSON" <<'PY'
import json, sys
src, dst, ip, port, a2s, maxp, guid, scenario, name, admins = sys.argv[1:11]
c = json.load(open(src))
c["bindAddress"] = "0.0.0.0"
c["bindPort"] = int(port)
c["publicAddress"] = ip
c["publicPort"] = int(port)
c.setdefault("a2s", {})["address"] = "0.0.0.0"
c["a2s"]["port"] = int(a2s)
g = c.setdefault("game", {})
g["name"] = name
g["scenarioId"] = scenario
g["maxPlayers"] = int(maxp)
# visible=false only hides it from the public browser; the room is registered either way and
# Direct Join still works. Left true so the friend can also just find it in the list.
g["visible"] = True
g["admins"] = json.loads(admins)
g["mods"] = [{"modId": guid, "name": "TBD_Framework"}]
json.dump(c, open(dst, "w"), indent=2)
PY
[ $? -eq 0 ] || { echo "ERROR: could not render $SERVER_JSON" >&2; exit 1; }
echo "    rendered $SERVER_JSON (mods=[$ADDON_GUID] admins=$ADMINS_JSON)"

if [ "${#ADMINS[@]}" -eq 0 ]; then
  echo ""
  echo "  NOTE: no --admin given, so game.admins[] is empty. TBD_AdminService.IsAdmin() resolves"
  echo "        from vanilla's SCR_PlayerListedAdminManagerComponent, which is populated ONLY"
  echo "        from game.admins[]. With none, every '#tbd' command answers 'TBD: admin only.'"
  echo "        and T-181.16's admin-respawn item cannot be reached. The 'passwordAdmin' field"
  echo "        is a DIFFERENT mechanism and does not feed that list."
  echo ""
fi

CMD_DISPLAY="./ArmaReforgerServer -addonsDir $RUN_DIR/addons -config $SERVER_JSON -profile $RUN_DIR/profile -maxFPS 60 -logStats 30000 -nothrow"

if [ "$DRY_RUN" -eq 1 ]; then
  echo ""
  echo "[dry-run] cd \"$SERVER_DIR\" && $CMD_DISPLAY"
  echo "[dry-run] would advertise: $LAN_IP:$GAME_PORT"
  exit 0
fi

# ── boot ───────────────────────────────────────────────────────────────────────────────────
# The pidfile is NOT deleted here. `assert_no_live_server` above already removed it, and only
# after confirming the group it named was dead; if it could not confirm that, this line was
# never reached. An unconditional `rm -f "$PIDFILE"` is what let a second invocation orphan the
# first one's server (T-608 / F5) — it threw away the only handle on a live process group.
LOGROOT="$RUN_DIR/profile/logs"
rm -rf "$LOGROOT"

trap 'echo ""; echo "==> stopping server"; if kill_run; then echo "==> stopped (process group confirmed gone)"; exit 0; fi; print_stray_warning; exit 1' INT TERM

TIMEOUT_PREFIX=""
[ -n "$RUN_TIMEOUT" ] && TIMEOUT_PREFIX="timeout -s TERM $RUN_TIMEOUT"

echo "==> booting (addon $ADDON_GUID, scenario $SCENARIO, mission $MISSION_ID)"
echo "    $CMD_DISPLAY"
hostrun env -C "$SERVER_DIR" setsid sh -c '
  echo $$ > "$1/server.pid"
  exec '"$TIMEOUT_PREFIX"' ./ArmaReforgerServer \
    -addonsDir "$1/addons" -config "$1/server.json" -profile "$1/profile" \
    -maxFPS 60 -logStats 30000 -nothrow
' _ "$RUN_DIR" >"$SRV_OUT" 2>&1 &
LAUNCHER=$!

# ── wait for the verdict ───────────────────────────────────────────────────────────────────
# Never `tail` a possibly-hanging stream; poll a file and read it. The three outcomes we poll
# for are the room registration, a config refusal, and an engine fatal.
#
# HOW LONG THIS TAKES IS NOT A RELIABLE NUMBER, and the comment here used to claim one
# ("~95 s measured, 300 s generous"). Measured on THIS machine, two boots minutes apart:
#
#   passing boot   `Server registered with address:` landed 13 s after `Starting RPL server`
#   failing boot   never, across the full 300 s — world long up, mission in LOBBY, vehicles
#                  spawned, and the room simply never registered
#
# Same binary, same config. So the wait is variable and the 300 s below is a bound on our
# patience, not an estimate of the engine's. What is worth printing is not a countdown against
# a fictional average but WHICH PHASE the boot is actually in, which `boot_phase` reads out of
# the log — "still compiling" and "world up, registration pending" are different problems and
# the old loop printed the same sentence for both, thirty times.
#
# Liveness is checked on the PROCESS GROUP from the pidfile, NOT on $LAUNCHER. Under the host
# bridge the local launcher returns almost immediately (world-boot.sh:809 records the same
# trap), so `kill -0 $LAUNCHER` reports "died" while the engine is still compiling scripts —
# measured here as a FAILED verdict 9 KB into a boot that was going fine. And it is checked
# every 10 s rather than every tick because each probe spawns a bridge process.

# The furthest milestone the log shows, newest first. Markers taken verbatim from a real boot.
# `grep -F` on the ones containing `[` — those are character classes to both ugrep and GNU grep.
boot_phase() {
  [ -f "$SRV_OUT" ] || { printf 'engine has not written anything yet'; return 0; }
  if grep -qF '[TBD][Stage] LOADING -> LOBBY' "$SRV_OUT" 2>/dev/null; then
    printf 'WORLD UP, mission already in LOBBY — the only thing missing is the backend room'
  elif grep -q 'Starting RPL server, listening on address' "$SRV_OUT" 2>/dev/null; then
    printf 'WORLD UP, replication listening — waiting on the backend room registration'
  elif grep -q 'Game::LoadEntities took' "$SRV_OUT" 2>/dev/null; then
    printf 'world entities loaded — waiting on replication, then the backend room'
  elif grep -q 'GameProject load' "$SRV_OUT" 2>/dev/null; then
    printf 'loading the world'
  elif grep -q 'Compiling Game scripts' "$SRV_OUT" 2>/dev/null; then
    printf 'compiling scripts'
  else
    printf 'engine starting'
  fi
}

# True once the world is demonstrably up. Distinguishes "it never got there" from "it got there
# and the registration hung", which need completely different diagnoses.
world_is_up() {
  [ -f "$SRV_OUT" ] || return 1
  grep -q 'Starting RPL server, listening on address' "$SRV_OUT" 2>/dev/null && return 0
  grep -q 'Game::LoadEntities took' "$SRV_OUT" 2>/dev/null && return 0
  grep -qF '[TBD][Stage] LOADING -> LOBBY' "$SRV_OUT" 2>/dev/null && return 0
  return 1
}

REGISTERED=""
FATAL=""
DIED=""
i=0
while [ "$i" -lt 600 ]; do
  i=$((i + 1))
  if [ -f "$SRV_OUT" ]; then
    if grep -q 'Server registered with address:' "$SRV_OUT" 2>/dev/null; then REGISTERED=1; break; fi
    if grep -qE 'There are errors in server config!|Unable to initialize the game|Unable to start replication' "$SRV_OUT" 2>/dev/null; then FATAL=1; break; fi
  fi
  if [ $((i % 20)) -eq 0 ]; then
    pgid="$(cat "$PIDFILE" 2>/dev/null)"
    if [ -n "$pgid" ]; then
      # Same rule as kill_run: only a CONFIRMED death breaks this loop. A bridge failure used
      # to land here as DIED, and then the kill path could not clean up what it had wrongly
      # declared dead — one unreachable probe manufactured both halves of the orphan.
      case "$(probe_group "$pgid")" in
        dead|zombie) DIED=1; break ;;
        unknown)     echo "    ... note: could not reach the host bridge to check the server; NOT reading that as death" ;;
      esac
    fi
    echo "    ... $((i / 2))s — $(boot_phase)"
  fi
  sleep 0.5
done

# Engine `(E)` lines, with the vanilla floor demoted rather than hidden. Measured on a PASSING
# boot of this same config, 2026-07-31: 79 `(E)`/`(F)` lines in total, of which 75 are the
# `DEFAULT`/`MATERIAL`/`RESOURCES` floor (70 of those are `DEFAULT (E): Trying to register a
# signal …` on vanilla vehicles). Four lines survive the demotion. The old code dumped the first
# 20 of the unsorted 79, which meant twenty lines of material and vehicle noise that are present
# when everything works — and the actual cause appeared in none of them.
dump_engine_errors() {
  local signal noise_n
  signal="$(grep -nE '\((E|F)\):' "$SRV_OUT" 2>/dev/null \
            | grep -vE '^[0-9]+:[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\(E\):' | head -20)"
  noise_n="$(grep -cE '^[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\(E\):' "$SRV_OUT" 2>/dev/null)"
  if [ -n "$signal" ]; then
    echo "--- engine errors worth reading ---" >&2
    printf '%s\n' "$signal" >&2
  else
    echo "--- no engine error line names a cause ---" >&2
  fi
  if [ "${noise_n:-0}" -gt 0 ]; then
    echo "    (${noise_n} further vanilla DEFAULT/MATERIAL/RESOURCES (E) lines suppressed — a" >&2
    echo "     passing boot of this config carries ~79 of them; they are the floor, not a clue)" >&2
  fi
  # This one is (E) and looks alarming and is not the problem. Say so where it will be read.
  if grep -qF '[TBD][Mission] backend refused the mission fetch' "$SRV_OUT" 2>/dev/null; then
    echo "    NOTE: '[TBD][Mission] backend refused the mission fetch — http=400' is BENIGN." >&2
    echo "          It means the mod could not fetch that id from the API and used the mission" >&2
    echo "          staged on disk instead — the --mission-file path working as designed. It is" >&2
    echo "          present on PASSING boots too. It has nothing to do with room registration." >&2
  fi
}

if [ -n "$FATAL" ] || [ -n "$DIED" ] || [ -z "$REGISTERED" ]; then
  kill_run || true
  echo "" >&2
  if [ -n "$DIED" ]; then
    echo "FAILED: the server process exited before registering a room." >&2
    echo "        phase reached: $(boot_phase)" >&2
    dump_engine_errors
  elif [ -n "$FATAL" ]; then
    echo "FAILED: the engine refused the config or could not start." >&2
    echo "        phase reached: $(boot_phase)" >&2
    dump_engine_errors
  elif world_is_up; then
    # The measured failure. The world came up, the mod loaded the mission, bodies spawned —
    # and the room never registered. Pointing at `(E)` lines here actively misleads, because
    # there is no `(E)` line for this: the engine logs nothing at all when the backend
    # handshake stalls. The evidence is an ABSENCE, so the diagnosis has to say so out loud.
    echo "FAILED: the world came up but the server never registered a backend room." >&2
    echo "        phase reached: $(boot_phase)" >&2
    echo "" >&2
    echo "  This is a REGISTRATION hang, not a load failure. Nothing was wrong with your mod," >&2
    echo "  your mission or your config — all of that demonstrably worked. What is missing is" >&2
    echo "  one line, 'BACKEND : Server registered with address:', and NO error line names it." >&2
    echo "" >&2
    echo "--- every BACKEND line in this boot (the answer is here, or its absence is) ---" >&2
    grep -nE '^[[:space:]]*BACKEND[[:space:]]*(\([EWF]\))?[[:space:]]*:' "$SRV_OUT" 2>/dev/null | tail -25 >&2
    echo "" >&2
    if grep -q 'Attempting online Game Config instead\.' "$SRV_OUT" 2>/dev/null; then
      echo "  Fingerprint: 'Attempting online Game Config instead.' with no BACKEND progress after" >&2
      echo "  it means the online handshake never completed. Measured 2026-07-31 on a boot that" >&2
      echo "  was otherwise perfect. It is upstream of us: check that this machine can reach" >&2
      echo "  Bohemia's backend at all, then simply run this script again — the same command" >&2
      echo "  registered in 13 s a few minutes later with nothing changed." >&2
      echo "" >&2
    fi
    dump_engine_errors
  else
    echo "FAILED: the server never registered a backend room, and never finished loading either." >&2
    echo "        phase reached: $(boot_phase)" >&2
    echo "        It did not get far enough for registration to be the suspect — read the phase." >&2
    dump_engine_errors
  fi
  echo "--- full output: $SRV_OUT" >&2
  echo "" >&2
  echo "The server binary exits 0 even when compilation fails — read the log, never \$?." >&2
  print_stray_warning || true
  exit 1
fi

# ── THE HARD GATE: did the LOCAL addon win, or the stale Workshop pak? ──────────────────────
# See the header. `-config` alone silently runs Workshop 1.0.1; if that copy wins here, every
# line below this point would be a true statement about the WRONG code, which is precisely the
# failure this script was written to make impossible.
assert_local_addon_won() {
  local loaded
  loaded="$(grep -A6 'Loaded addons:' "$SRV_OUT" | grep "guid: '$ADDON_GUID'" | tail -1)"
  if [ -z "$loaded" ]; then
    echo "FAILED: the engine never reported loading addon $ADDON_GUID at all." >&2
    grep -nE 'Loaded addons:|Available addons:|gproj:' "$SRV_OUT" | head -20 >&2
    return 1
  fi
  case "$loaded" in
    *"$RUN_DIR/addons/tbd-framework/addon.gproj"*) return 0 ;;
  esac
  echo "" >&2
  echo "FAILED: the STALE Workshop copy won, not your checkout." >&2
  echo "  loaded: $loaded" >&2
  echo "  wanted: $RUN_DIR/addons/tbd-framework/addon.gproj" >&2
  echo "" >&2
  echo "  tbd-framework is published unlisted under the same id as the local gproj GUID," >&2
  echo "  so the engine can satisfy game.mods[] from the Workshop without ever touching your" >&2
  echo "  source. That build is version 1.0.1 and months old. Delete the cached copy and retry:" >&2
  echo "      rm -rf '$RUN_DIR/profile/addons/TBDFramework_$ADDON_GUID'" >&2
  return 1
}
if ! assert_local_addon_won; then kill_run || true; print_stray_warning || true; exit 1; fi

REG_LINE="$(grep -m1 'Server registered with address:' "$SRV_OUT")"
REG_ADDR="$(printf '%s' "$REG_LINE" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+:[0-9]+')"
JOIN_CODE="$(grep -m1 'Direct Join Code:' "$SRV_OUT" | grep -oE '[0-9]{6,}')"
LOADED_LINE="$(grep -A6 'Loaded addons:' "$SRV_OUT" | grep "guid: '$ADDON_GUID'" | tail -1 | sed 's/^[[:space:]]*//')"

cat <<EOF

================================================================================
  SERVER UP — second client joins with:

    Multiplayer -> Direct Join -> ${REG_ADDR:-$LAN_IP:$GAME_PORT}
    or Direct Join Code:          ${JOIN_CODE:-<none printed>}

  (the join code is re-minted on every boot — read it from THIS run, not a doc)
================================================================================
  proof, from this boot:
    $REG_LINE
    $LOADED_LINE
================================================================================

  THE CLIENT SIDE IS NOT PROVEN BY THIS SCRIPT. The server advertises
  game.mods[] = [$ADDON_GUID], and the joining client resolves that id from the
  WORKSHOP, where it is pinned at the stale version 1.0.1. The server is running
  your checkout. If the friend's client refuses the join on a version/content
  mismatch, or joins and behaves like older code, that skew is the first suspect
  — re-publish tbd-framework from Workbench before blaming the mod.

  Ctrl-C stops the server. Wait for '==> stopped (process group confirmed gone)'
  before starting another one — that line means the group was PROVED dead, not
  assumed dead. If you get a STRAY SERVER block instead, run the command it prints:
  a survivor holds $GAME_PORT/$A2S_PORT and the next boot will fail on the port.
EOF

if [ "${#ADMINS[@]}" -gt 0 ]; then
  echo "  admins configured: ${ADMINS[*]}"
  echo "  (the engine schema-validated these; whether a given id maps to the human who"
  echo "   connects is only observable once they DO connect — check '#tbd' in chat.)"
  echo ""
fi

# Foreground from here so the operator watches the live log; the trap stops the server.
CONSOLE="$(ls -1d "$LOGROOT"/logs_* 2>/dev/null | tail -1)/console.log"
if [ -f "$CONSOLE" ]; then
  echo "==> tailing $CONSOLE"
  tail -f "$CONSOLE" &
else
  echo "==> tailing $SRV_OUT"
  tail -f "$SRV_OUT" &
fi
TAILPID=$!

# Wait on the SERVER's process group, NOT on $LAUNCHER.
#
# `wait "$LAUNCHER"` is what this line said first, and it was a live instance of the defect this
# whole script exists to prevent: under the host bridge the local launcher returns the moment the
# server is `setsid`-detached, so `wait` fell straight through, `kill_run` ran, and the script
# printed "SERVER UP" and then killed the server about five seconds later. It exited 0. The only
# symptom was a friend who could not join a server the banner had just declared up.
#
# The probe is the tri-state one for the same reason kill_run uses it: a bridge hiccup here
# would otherwise fall straight out of this loop and report a running server as exited.
while :; do
  pgid="$(cat "$PIDFILE" 2>/dev/null)"
  [ -n "$pgid" ] || break
  case "$(probe_group "$pgid")" in
    dead|zombie) break ;;
    unknown)     echo "    (host bridge unreachable — still assuming the server is UP; it is not evidence of exit)" ;;
  esac
  sleep 5
done

kill "$TAILPID" 2>/dev/null
echo ""
if kill_run; then
  echo "==> server stopped (process group confirmed gone)"
  exit 0
fi
print_stray_warning
exit 1
