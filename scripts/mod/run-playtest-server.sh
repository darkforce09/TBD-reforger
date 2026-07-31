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
# Usage:
#   bash scripts/mod/run-playtest-server.sh --mission-id=<id> [options]
#   bash scripts/mod/run-playtest-server.sh --mission-id=<id> --admin=<identityId> --dry-run
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
    # Prints the Usage + Options comment block above verbatim. Bounded by `set -uo pipefail`'s
    # line so it cannot silently drift into code if the header grows.
    -h|--help)        sed -n '/^# Usage:/,/^set -uo pipefail/p' "$0" | grep -E '^#' | sed 's/^# \?//'; exit 0 ;;
    *)                usage_fail "unknown argument: $arg" ;;
  esac
done

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
LOGROOT="$RUN_DIR/profile/logs"
rm -rf "$LOGROOT"
PIDFILE="$RUN_DIR/server.pid"
rm -f "$PIDFILE"
SRV_OUT="$RUN_DIR/server.out"

# Same kill discipline as world-boot.sh:423 — the launcher runs under `setsid`, the recorded PID
# is a PROCESS GROUP LEADER, and we signal the whole group. Deliberately NOT a name match: a
# broad `pkill -f ArmaReforgerServer` would also kill the operator's own dev server, and (measured)
# the bridge's own `sh -c` command line contains that string, so it kills the caller too.
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
trap 'echo ""; echo "==> stopping server"; kill_run; exit 0' INT TERM

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
# for are the room registration, a config refusal, and an engine fatal. 300 s is generous: the
# measured path to `Server registered with address:` is ~95 s on this machine, most of it the
# script compile and the world load.
#
# Liveness is checked on the PROCESS GROUP from the pidfile, NOT on $LAUNCHER. Under the host
# bridge the local launcher returns almost immediately (world-boot.sh:809 records the same
# trap), so `kill -0 $LAUNCHER` reports "died" while the engine is still compiling scripts —
# measured here as a FAILED verdict 9 KB into a boot that was going fine. And it is checked
# every 10 s rather than every tick because each probe spawns a bridge process.
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
    if [ -n "$pgid" ] && ! hostrun kill -0 -- "-$pgid" >/dev/null 2>&1; then DIED=1; break; fi
    echo "    ... $((i / 2))s, still booting (compile + world load take ~95 s)"
  fi
  sleep 0.5
done

if [ -n "$FATAL" ] || [ -n "$DIED" ] || [ -z "$REGISTERED" ]; then
  kill_run
  echo "" >&2
  if [ -n "$DIED" ]; then
    echo "FAILED: the server process exited before registering a room." >&2
  else
    echo "FAILED: the server never registered a backend room." >&2
  fi
  echo "--- offending lines ---" >&2
  grep -nE '\(E\):|\(F\):|There are errors in server config!|Unable to' "$SRV_OUT" 2>/dev/null | head -20 >&2
  echo "--- full output: $SRV_OUT" >&2
  echo "" >&2
  echo "The server binary exits 0 even when compilation fails — read the log, never \$?." >&2
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
if ! assert_local_addon_won; then kill_run; exit 1; fi

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

  Ctrl-C stops the server.
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
while :; do
  pgid="$(cat "$PIDFILE" 2>/dev/null)"
  [ -n "$pgid" ] || break
  hostrun kill -0 -- "-$pgid" >/dev/null 2>&1 || break
  sleep 5
done

kill "$TAILPID" 2>/dev/null
kill_run
echo ""
echo "==> server exited"
