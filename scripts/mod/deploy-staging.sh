#!/usr/bin/env bash
# Rsync TBD platform to 192.168.0.140, rebuild API, refresh profile, restart game server.
#
# Prereqs (dev PC):
#   cp scripts/deploy/deploy.env.example scripts/deploy/deploy.env   # fill SSH + token
#   (tbd-schema is npm-free since T-165.9 — schema gates run via `cargo xtask`)
#
# Usage:
#   bash scripts/mod/deploy-staging.sh                       # mode from deploy.env (default: addons)
#   bash scripts/mod/deploy-staging.sh --dry-run
#
#   # joinable server (after publishing tbd-framework to the Workshop):
#   TBD_SERVER_MODE=config TBD_WORKSHOP_MOD_ID=<workshopModId> bash scripts/mod/deploy-staging.sh
#
#   # render server.config.json LOCALLY and exit — no rsync, no ssh, no deploy (T-288):
#   TBD_SERVER_MODE=config TBD_MODPACK_JSON=pack.json \
#     bash scripts/mod/deploy-staging.sh --render-only /tmp/server.config.json
#
#   # render the host control agent LOCALLY and exit — no rsync, no ssh (T-289):
#   bash scripts/mod/deploy-staging.sh --render-agent /tmp/agent
#
#   # render the agent AND drive it against a stub systemctl — no rsync, no ssh (T-289):
#   bash scripts/mod/deploy-staging.sh --agent-selftest /tmp/agent-selftest
#
# Never rsyncs to /home/sam/prairielearn/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
# shellcheck source=lib/gate-grep.sh
source "$SCRIPT_DIR/lib/gate-grep.sh"
ENV_FILE="$DEPLOY_ENV"
DRY_RUN=0
RENDER_ONLY_OUT=""
RENDER_AGENT_OUT=""
AGENT_SELFTEST_OUT=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --render-only)
      # T-288: render the server config to a LOCAL path and exit 0 before any
      # rsync/ssh runs. This is the only way to exercise the render half without
      # touching a real server, and it is what the perturbation gate drives.
      shift
      RENDER_ONLY_OUT="${1:-}"
      if [ -z "$RENDER_ONLY_OUT" ]; then
        echo "--render-only requires an output path" >&2
        exit 2
      fi
      ;;
    --render-agent)
      # T-289: render the host control agent + its systemd units into a LOCAL
      # directory and exit 0, before any rsync/ssh. Same split T-288 made for the
      # server config: the artefact is inspectable without deploying it.
      shift
      RENDER_AGENT_OUT="${1:-}"
      if [ -z "$RENDER_AGENT_OUT" ]; then
        echo "--render-agent requires an output directory" >&2
        exit 2
      fi
      ;;
    --agent-selftest)
      # T-289: render the agent, then RUN it against a stub systemctl whose
      # answers this script controls, and assert the agent reports the unit's
      # real state. See agent_selftest() for why this is the whole point.
      shift
      AGENT_SELFTEST_OUT="${1:-}"
      if [ -z "$AGENT_SELFTEST_OUT" ]; then
        echo "--agent-selftest requires a working directory" >&2
        exit 2
      fi
      ;;
    -h|--help)
      echo "Usage: deploy-staging.sh [--dry-run] [--render-only <path>]"
      echo "                         [--render-agent <dir>] [--agent-selftest <dir>]"
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done

# ── T-289: the host control agent ────────────────────────────────────────────
#
# WHAT THIS IS. `POST /api/v1/admin/servers/:id/rcon` answers 503 RCON_NO_TRANSPORT
# (handlers/admin.rs:551) because the API has no channel to the game host. This section
# renders the host half of that channel. The API half is a separate slice — see
# §"WHAT THE API SLICE MUST BUILD" below.
#
# ── THE FACT THAT DECIDES THE DESIGN ─────────────────────────────────────────
#
# T-269 recorded "The game server is a **separate host**" (admin.rs:517). That is true of
# the DEVELOPER'S PC and false of the API. Re-measured on main 2026-07-31:
#
#   docs/mod/STAGING-SERVER.md:3   "API + Postgres and Arma Reforger dedicated server
#                                   on `sam@192.168.0.140`"      — ONE box
#   scripts/deploy/deploy.env.example:17  TBD_SSH_HOST=sam@192.168.0.140
#                                   — ONE ssh host for BOTH deploy scripts
#   docs/website/HOME_SERVER.md:282-304   the API is `~/.config/systemd/user/
#                                   tbd-website-api.service`, ExecStart=…/target/release/api
#   scripts/deploy/deploy-website.sh:230  `systemctl --user restart tbd-website-api.service`
#   this script (below)             `systemctl --user restart tbd-reforger.service`
#   this script's TBD_BACKEND_URL   http://127.0.0.1:8080 — the mod reaches the API on LOOPBACK
#
# So the API process and the game server are SIBLING `systemctl --user` units, same uid
# (`sam`), same user systemd manager, same $XDG_RUNTIME_DIR. Only Postgres is in Docker
# (deploy-website.sh runs `compose up -d postgres`, and the compose `api` service is behind
# an opt-in `--profile api`). STAGING-SERVER.md:3's "(Docker)" reads as if the API were
# containerised; deploy-website.sh and HOME_SERVER Phase D are the authoritative pair.
#
# THAT COLLAPSES THE CREDENTIAL PROBLEM. T-269 asked for "a migration adding an agent
# endpoint and a secret reference" because it assumed a network hop. Across a same-uid UNIX
# socket the OPERATING SYSTEM is the credential: a socket at $XDG_RUNTIME_DIR with
# SocketMode=0600 can be opened by exactly one uid, and that uid is the API. There is no
# shared secret to store, rotate, or leak, and nothing to add to `servers` for this
# deployment. (A SECOND game host would need the migration — specified below.)
#
# ── WHAT WAS REJECTED ────────────────────────────────────────────────────────
#
# * SSH from an axum handler (`Command::new("ssh") … systemctl --user restart`). T-269
#   rejected this and the judgement stands and re-verifies: `send_rcon` is gated by
#   `AdminUser`, a Discord-OAuth session, and `RconCommand::Custom` (admin.rs:493-510)
#   carries operator-supplied free text. That is remote code execution with an admin
#   checkbox in front of it. It is also not even possible on the box: `scripts/deploy/
#   deploy.env` is gitignored AND rsync-excluded (see --exclude list below), so the
#   credential exists only on a developer's PC. A channel that works because someone's SSH
#   key happens to be present is not a channel.
# * BattlEye / Reforger RCON over UDP. Re-measured: `ss -lntu` binds only :8080 / :3000 /
#   :5434 (+ :5432) — 19999 is never bound; render_server_config() below emits NO `rcon`
#   key and `"battlEye": false`. Enabling it would mean shipping an admin password in the
#   config and speaking a protocol we cannot exercise here. DECISIVE: RCON only reaches a
#   server that is ALREADY RUNNING. It structurally cannot do `start`, which is half this
#   ticket's title.
# * A queued-command table the mod polls. Needs a migration plus mod-side polling that does
#   not exist — and a dead server polls nothing, so again it cannot `start`.
#
# ── WHY THE AGENT RE-READS THE UNIT, WHICH IS THE ENTIRE POINT ───────────────
#
# `systemctl --user restart tbd-reforger.service` EXITS 0 OVER A SERVER THAT IS DEAD.
# This is not hypothetical on this host — docs/mod/STAGING-SERVER.md:246-250 documents it:
# with `-a2sPort` equal to `-bindPort` the engine logs "Unable to start replication" →
# "Game destroyed" and "exits status 0, so `Restart=on-failure` does NOT restart it".
# The deploy path below has always run that restart and then `sleep 8` without ever
# checking — a tool reporting success over a server it never examined, which is this
# program's signature defect, already live in this file.
#
# So the agent NEVER derives its answer from the exit status of the verb. It runs the
# verb, waits out the dwell, and RE-READS the unit's LoadState/ActiveState from systemd.
# `accepted` means the unit was observed in the state the action intended. That is what
# lets T-269's 503 become an honest 202.
: "${TBD_AGENT_UNIT:=tbd-reforger.service}"
: "${TBD_AGENT_SOCKET:=tbd-reforger-agent.sock}"
# Seconds to let a started unit prove it stays up before the state is read. Matches the
# `sleep 8` the deploy already uses for the same reason. The selftest drives this to 0.
: "${TBD_AGENT_DWELL_S:=8}"
# Absolute path the agent script lands on ON THE HOST. Referenced by ExecStart=, which
# systemd requires to be absolute, so this cannot be derived at render time from a relative
# path. Default matches the /home/sam/tbd/ prefix deploy-website.sh already enforces.
: "${TBD_AGENT_REMOTE_PATH:=/home/sam/tbd/tbd-reforger-agent.sh}"
# Install the agent as part of a real deploy. DEFAULT OFF, deliberately: the install step
# mutates a live host and T-289 could not exercise it (the operator is using that stack, and
# the slice brief forbids running the deploy path). The RENDER is proven by --agent-selftest;
# the INSTALL is not, so it must be opted into by someone watching it. Turning this on before
# the API side exists gains nothing anyway — nothing would connect to the socket.
: "${TBD_INSTALL_AGENT:=0}"

# Unit/socket names are interpolated into systemd unit files. Keep them to a charset that
# cannot carry a newline, a quote or a directive — fail closed rather than emit a unit file
# whose meaning depends on someone's env var.
validate_agent_names() {
  case "$TBD_AGENT_UNIT" in
    *[!A-Za-z0-9._@-]*|"")
      echo "FAIL: TBD_AGENT_UNIT='$TBD_AGENT_UNIT' — only A-Za-z0-9._@- allowed." >&2
      exit 1 ;;
  esac
  case "$TBD_AGENT_SOCKET" in
    *[!A-Za-z0-9._-]*|"")
      echo "FAIL: TBD_AGENT_SOCKET='$TBD_AGENT_SOCKET' — only A-Za-z0-9._- allowed." >&2
      exit 1 ;;
  esac
}

# Render the agent + its two systemd units into the LOCAL directory $1.
#
# The agent is a pure stdin->stdout filter and holds NO socket code, because bash cannot
# bind a UNIX socket without pulling in socat/nc. systemd's `Accept=yes` socket activation
# supplies the connection on stdin, which removes the dependency instead of asserting it.
#
# The unit it controls arrives via `Environment=` in the .service, not by substitution into
# the script — so the agent script is byte-identical on every host and the per-server
# addressing lives in the systemd unit, which is systemd's own place for it.
render_agent_files() {
  local out="$1"
  validate_agent_names
  mkdir -p "$out"

  cat > "$out/tbd-reforger-agent.sh" <<'AGENT_EOF'
#!/usr/bin/env bash
# TBD Reforger host control agent (T-289) — RENDERED by scripts/mod/deploy-staging.sh.
# Do not edit on the host; edit the renderer and redeploy.
#
# Contract: read ONE line from stdin, write ONE line of JSON to stdout.
#
#   in : status | start | stop | restart
#   out: {"ok":<bool>,"action":"<verb>","result":"<r>","state":"<s>","detail":"<text>"}
#
#   result  accepted    the verb ran AND the unit was observed in the intended state
#           rejected    the verb is unknown, or it ran and the unit did NOT get there
#           unreachable systemd could not be reached, or the unit is not installed
#   state   systemd ActiveState as observed AFTER the action: active | inactive |
#           failed | activating | deactivating | reloading | unknown
#
# SECURITY. There is deliberately NO passthrough verb. The request is filtered to [a-z]
# and then matched against a fixed four-element set, so no operator-supplied text — and no
# shell metacharacter — can reach a command. `custom` and `change_map` from RconInput have
# no representation here BY DESIGN; see the scope note in deploy-staging.sh.
set -uo pipefail

UNIT="${TBD_AGENT_UNIT:-}"
SYSTEMCTL="${TBD_AGENT_SYSTEMCTL:-systemctl}"
DWELL="${TBD_AGENT_DWELL_S:-8}"
ACTION="unknown"

# The only variable content in the output is $detail. Restrict it to a charset containing
# no JSON metacharacter, so this hand-rolled JSON cannot emit an invalid document — the
# failure deploy-staging.sh's own header warns about. Every other field is from a fixed set.
emit() {
  local ok="$1" result="$2" state="$3" detail="$4"
  detail="$(printf '%s' "$detail" | tr -cd 'A-Za-z0-9 ._:/@=-' | cut -c1-200)"
  printf '{"ok":%s,"action":"%s","result":"%s","state":"%s","detail":"%s"}\n' \
    "$ok" "$ACTION" "$result" "$state" "$detail"
}

# Read LoadState and ActiveState in one call and parse BY KEY — `systemctl show` does not
# promise the properties come back in the order they were asked for.
#
# LoadState matters on its own: `systemctl show` on a unit that does not exist still exits
# 0 and reports ActiveState=inactive. Trusting ActiveState alone would report a UNINSTALLED
# server as merely "stopped", which is the same class of lie this agent exists to end.
read_state() {
  local raw line load="" active=""
  raw="$("$SYSTEMCTL" --user show --property=LoadState --property=ActiveState -- "$UNIT" 2>/dev/null)" || return 1
  while IFS= read -r line; do
    case "$line" in
      LoadState=*)   load="${line#LoadState=}" ;;
      ActiveState=*) active="${line#ActiveState=}" ;;
    esac
  done <<< "$raw"
  [ -n "$load" ] || return 1
  case "$load" in
    loaded) ;;
    *) printf 'NOTLOADED %s' "$load"; return 0 ;;
  esac
  case "$active" in
    active|inactive|failed|activating|deactivating|reloading) printf 'OK %s' "$active" ;;
    *) printf 'OK unknown' ;;
  esac
}

read -r request || request=""
# Filter to lowercase letters BEFORE matching: strips CR from a \r\n client, trailing
# whitespace, and anything else. "rm -rf /" becomes "rmrf", which is not in the set below.
candidate="$(printf '%s' "$request" | tr -cd 'a-z')"
case "$candidate" in
  status|start|stop|restart) ACTION="$candidate" ;;
  *) emit false rejected unknown "unknown action"; exit 0 ;;
esac

if [ -z "$UNIT" ]; then
  emit false unreachable unknown "TBD_AGENT_UNIT not set in the service unit"
  exit 0
fi
if ! command -v "$SYSTEMCTL" >/dev/null 2>&1; then
  emit false unreachable unknown "systemctl not available"
  exit 0
fi

probe="$(read_state)" || { emit false unreachable unknown "systemd did not answer"; exit 0; }
case "$probe" in
  NOTLOADED*) emit false unreachable unknown "unit not installed: ${probe#NOTLOADED }"; exit 0 ;;
esac

# `status` only observes; the caller reads `state` for the answer.
if [ "$ACTION" = "status" ]; then
  emit true accepted "${probe#OK }" "observed"
  exit 0
fi

verb_rc=0
"$SYSTEMCTL" --user "$ACTION" -- "$UNIT" >/dev/null 2>&1 || verb_rc=$?

# THE DWELL. Not politeness — a Reforger server that mis-starts exits 0 a few seconds in
# (STAGING-SERVER.md:246-250), so a state read taken immediately after `start` returns
# `active` for a server that is already dying. Reading the state only AFTER the dwell is
# what makes `accepted` mean something.
if [ "$ACTION" != "stop" ] && [ "$DWELL" != "0" ]; then
  sleep "$DWELL"
fi

probe="$(read_state)" || { emit false unreachable unknown "systemd did not answer after $ACTION"; exit 0; }
case "$probe" in
  NOTLOADED*) emit false unreachable unknown "unit vanished during $ACTION"; exit 0 ;;
esac
state="${probe#OK }"

# The verdict is the OBSERVED state, never $verb_rc. A zero exit over a dead unit is
# exactly the defect this agent exists to stop reporting.
case "$ACTION" in
  start|restart)
    if [ "$state" = "active" ]; then
      emit true accepted "$state" "unit active after $ACTION"
    else
      emit false rejected "$state" "unit is $state after $ACTION; systemctl rc=$verb_rc"
    fi ;;
  stop)
    if [ "$state" = "inactive" ] || [ "$state" = "failed" ]; then
      emit true accepted "$state" "unit stopped"
    else
      emit false rejected "$state" "unit is $state after stop; systemctl rc=$verb_rc"
    fi ;;
esac
AGENT_EOF
  chmod +x "$out/tbd-reforger-agent.sh"

  # SocketMode=0600 in %t ($XDG_RUNTIME_DIR, mode 0700, owned by the run user) IS the
  # credential: one uid can open it, and that uid is the API's. Accept=yes gives each
  # connection its own short-lived instance, so a wedged request cannot block the next.
  cat > "$out/tbd-reforger-agent.socket" <<EOF
[Unit]
Description=TBD Reforger host control agent socket (T-289)
Documentation=man:systemd.socket(5)

[Socket]
ListenStream=%t/$TBD_AGENT_SOCKET
SocketMode=0600
Accept=yes

[Install]
WantedBy=sockets.target
EOF

  cat > "$out/tbd-reforger-agent@.service" <<EOF
[Unit]
Description=TBD Reforger host control agent connection (T-289)
Documentation=man:systemd.socket(5)

[Service]
Type=oneshot
ExecStart=$TBD_AGENT_REMOTE_PATH
Environment=TBD_AGENT_UNIT=$TBD_AGENT_UNIT
Environment=TBD_AGENT_DWELL_S=$TBD_AGENT_DWELL_S
StandardInput=socket
StandardOutput=socket
StandardError=journal
EOF
}

# Structural check of a rendered agent. Same posture as validate_server_config(): re-read
# the artefact and pin the invariants, rather than trusting that the heredoc above ran.
validate_agent_files() {
  local d="$1" fail=0
  gate_require "agent script missing its state re-read (read_state)" \
    -F 'read_state' "$d/tbd-reforger-agent.sh" || fail=1
  gate_require "agent must gate on LoadState, not ActiveState alone" \
    -F 'LoadState' "$d/tbd-reforger-agent.sh" || fail=1
  gate_require "socket must be 0600 — the file mode IS the credential" \
    -F 'SocketMode=0600' "$d/tbd-reforger-agent.socket" || fail=1
  gate_require "socket must live in %t (\$XDG_RUNTIME_DIR)" \
    -F 'ListenStream=%t/' "$d/tbd-reforger-agent.socket" || fail=1
  gate_require "service must name the unit it controls" \
    -F "Environment=TBD_AGENT_UNIT=$TBD_AGENT_UNIT" "$d/tbd-reforger-agent@.service" || fail=1
  gate_require "service must take the connection on stdin (Accept=yes contract)" \
    -F 'StandardInput=socket' "$d/tbd-reforger-agent@.service" || fail=1
  # The agent must never grow a passthrough. `custom` is operator-supplied free text and
  # the only reason this channel is safe behind a session cookie is that it cannot carry it.
  # Pin the accepted verb set literally rather than banning the WORD "custom" — the script's
  # own security comment says the word, and a ban that trips on its own documentation is a
  # gate nobody can keep green honestly.
  gate_require "agent must accept exactly the four process verbs" \
    -F 'status|start|stop|restart) ACTION="$candidate" ;;' "$d/tbd-reforger-agent.sh" || fail=1
  gate_ban "agent must not grow a custom/passthrough case arm" \
    -F 'custom)' "$d/tbd-reforger-agent.sh" || fail=1
  gate_ban "agent must never eval a request" \
    'eval[[:space:]]' "$d/tbd-reforger-agent.sh" || fail=1
  # NOTE: no -E flag — gate_grep's default engine is already ERE, and passing -E would be
  # consumed as the PATTERN (it only parses -F/-i), turning the file into a second pattern.
  gate_ban "agent must not derive its verdict from the systemctl exit status" \
    'verb_rc.*(-eq|==)' "$d/tbd-reforger-agent.sh" || fail=1
  [ "$fail" -eq 0 ] || return 1
  echo "  agent VALID: unit=$TBD_AGENT_UNIT socket=%t/$TBD_AGENT_SOCKET dwell=${TBD_AGENT_DWELL_S}s"
}

# Drive the rendered agent against a STUB systemctl whose answers this function controls.
#
# WHY A STUB AND NOT THE REAL ONE. The real `systemctl --user` on a dev box has no
# tbd-reforger.service, so every case would collapse to "unreachable" and prove nothing;
# and the one host that does have it is the live staging server, which this script must not
# touch. The stub lets the interesting states — a unit that reports `active`, one that is
# `failed`, one that is not installed — all be produced on demand, locally.
#
# WHAT IT PROVES. Case 4 is the ticket: `systemctl restart` EXITS 0 while the unit is
# `failed`. An agent that trusted that exit status would answer `accepted` over a dead
# server. The assertion demands `rejected` + `state=failed`, so the honest answer passes
# and the signature defect fails.
agent_selftest() {
  local d="$1" pass=0 fail=0
  rm -rf "$d"
  mkdir -p "$d/bin"
  TBD_AGENT_DWELL_S=0 render_agent_files "$d"

  # Stub systemctl. STUB_LOAD/STUB_ACTIVE are the unit's state; STUB_VERB_RC is what the
  # verb returns — deliberately independent, so "verb says OK, unit is dead" is expressible.
  cat > "$d/bin/systemctl" <<'STUB_EOF'
#!/usr/bin/env bash
for a in "$@"; do
  if [ "$a" = "show" ]; then
    printf 'LoadState=%s\nActiveState=%s\n' "${STUB_LOAD:-loaded}" "${STUB_ACTIVE:-inactive}"
    exit 0
  fi
done
exit "${STUB_VERB_RC:-0}"
STUB_EOF
  chmod +x "$d/bin/systemctl"

  # name | request | STUB_LOAD | STUB_ACTIVE | STUB_VERB_RC | expected result | expected state
  local -a cases=(
    "status of a running unit|status|loaded|active|0|accepted|active"
    "status of a stopped unit|status|loaded|inactive|0|accepted|inactive"
    "restart that really came up|restart|loaded|active|0|accepted|active"
    "restart that exits 0 over a DEAD unit|restart|loaded|failed|0|rejected|failed"
    "start that never came up|start|loaded|inactive|0|rejected|inactive"
    "stop that really stopped|stop|loaded|inactive|0|accepted|inactive"
    "unit not installed|status|not-found|inactive|0|unreachable|unknown"
    "unit masked|restart|masked|inactive|0|unreachable|unknown"
    "garbage verb is refused|rm -rf /|loaded|active|0|rejected|unknown"
    "empty request is refused||loaded|active|0|rejected|unknown"
  )

  local c name req load active rc want_result want_state out got_result got_state
  for c in "${cases[@]}"; do
    IFS='|' read -r name req load active rc want_result want_state <<< "$c"
    out="$(printf '%s\n' "$req" | \
      PATH="$d/bin:$PATH" TBD_AGENT_UNIT="$TBD_AGENT_UNIT" TBD_AGENT_DWELL_S=0 \
      STUB_LOAD="$load" STUB_ACTIVE="$active" STUB_VERB_RC="$rc" \
      bash "$d/tbd-reforger-agent.sh" 2>/dev/null)"
    got_result="$(printf '%s' "$out" | sed -n 's/.*"result":"\([a-z]*\)".*/\1/p')"
    got_state="$(printf '%s' "$out" | sed -n 's/.*"state":"\([a-z-]*\)".*/\1/p')"
    if [ "$got_result" = "$want_result" ] && [ "$got_state" = "$want_state" ]; then
      echo "  PASS  $name -> $got_result/$got_state"
      pass=$((pass + 1))
    else
      echo "  FAIL  $name"
      echo "        want result=$want_result state=$want_state"
      echo "        got  result=$got_result state=$got_state"
      echo "        raw  $out"
      fail=$((fail + 1))
    fi
  done

  # The output must be a single valid JSON object, not just something that greps right.
  if command -v python3 >/dev/null 2>&1; then
    out="$(printf 'status\n' | PATH="$d/bin:$PATH" TBD_AGENT_UNIT="$TBD_AGENT_UNIT" \
      STUB_LOAD=loaded STUB_ACTIVE=active bash "$d/tbd-reforger-agent.sh" 2>/dev/null)"
    if printf '%s' "$out" | python3 -c 'import json,sys; d=json.load(sys.stdin); assert set(d)=={"ok","action","result","state","detail"}, d' 2>/dev/null; then
      echo "  PASS  response parses as JSON with exactly the contract keys"
      pass=$((pass + 1))
    else
      echo "  FAIL  response is not valid contract JSON: $out"
      fail=$((fail + 1))
    fi
  fi

  echo
  if [ "$fail" -ne 0 ]; then
    echo "AGENT SELFTEST: $pass passed, $fail FAILED"
    return 1
  fi
  echo "AGENT SELFTEST: $pass passed, 0 failed"
  validate_agent_files "$d"
}

if [ -n "$RENDER_AGENT_OUT" ]; then
  echo "==> render host control agent (local only, no deploy) -> $RENDER_AGENT_OUT"
  render_agent_files "$RENDER_AGENT_OUT"
  validate_agent_files "$RENDER_AGENT_OUT"
  exit 0
fi

if [ -n "$AGENT_SELFTEST_OUT" ]; then
  echo "==> agent selftest (local only, no deploy) -> $AGENT_SELFTEST_OUT"
  agent_selftest "$AGENT_SELFTEST_OUT"
  exit 0
fi

if [ ! -f "$ENV_FILE" ]; then
  echo "Missing $ENV_FILE — copy from scripts/deploy/deploy.env.example" >&2
  exit 1
fi
# shellcheck source=/dev/null
source "$ENV_FILE"

: "${TBD_SSH_HOST:?TBD_SSH_HOST required in deploy.env}"
: "${TBD_REMOTE_DIR:?TBD_REMOTE_DIR required}"
: "${TBD_PROFILE_DIR:?TBD_PROFILE_DIR required}"
: "${TBD_ADDONS_STAGING:?TBD_ADDONS_STAGING required}"
: "${TBD_GAME_SERVER_TOKEN:?TBD_GAME_SERVER_TOKEN required}"
: "${TBD_MISSION_ID:=msn_8f3a2c}"
: "${TBD_EVENT_ID:=b0000000-0000-4000-8000-000000000001}"
: "${TBD_BACKEND_URL:=http://127.0.0.1:8080}"
: "${TBD_ADDON_GUID:=B2C3D4E5F6A78901}"
: "${TBD_SCENARIO:={69A85365FC09E2CA}Missions/TBD_Dev_POC.conf}"
: "${TBD_BIND_IP:=192.168.0.140}"
: "${TBD_SERVER_DIR:=/home/sam/steam/arma-reforger-server}"

# Server launch mode:
#   addons  — -server + -addons (local unpublished mod). Runs headless for log
#             verification (mission load, 18x slot spawn, Stage -> LOBBY) but is
#             NOT Direct-Joinable: -server+-addons registers no backend room.
#   config  — -config (server config JSON). Registers a backend room ("Server
#             registered with address:" / "Direct Join Code:") and IS joinable.
#             Requires the mod to be PUBLISHED to the Workshop (config game.mods[]
#             only loads Workshop content; -config is incompatible with -addons),
#             so TBD_WORKSHOP_MOD_ID must be set to the real Workshop modId.
: "${TBD_SERVER_MODE:=addons}"
: "${TBD_WORKSHOP_MOD_ID:=}"
: "${TBD_PUBLIC_ADDRESS:=${TBD_BIND_IP}}"
: "${TBD_GAME_PORT:=2001}"
: "${TBD_A2S_PORT:=17777}"          # MUST differ from TBD_GAME_PORT or replication fails
: "${TBD_SERVER_NAME:=TBD Staging POC}"
: "${TBD_ADMIN_PASSWORD:=tbd-admin}"
: "${TBD_MAX_PLAYERS:=64}"
: "${TBD_ADMIN_IDENTITY_IDS:=}"   # comma-separated identityIds → in-game admins (#tbd commands)
: "${TBD_SERVER_CONFIG_REMOTE:=$(dirname "$TBD_PROFILE_DIR")/server.config.json}"

# ── T-288: where game.mods[] comes from ──────────────────────────────────────
#
# Before T-288 this script hardcoded ONE mod — `{"modId": "$TBD_WORKSHOP_MOD_ID",
# "name": "TBD_Framework"}` — and never read the `modpacks` / `modpack_mods` tables.
# A modpack authored on the website therefore had no path to a running server.
#
# THE SOURCE IS THE API, and specifically the bytes of
#   GET /api/v1/modpacks/current   (apps/website/api/src/app.rs `/modpacks/current`
#                                   -> handlers/modpacks.rs::get_current_modpack)
# whose `mods[]` rows carry exactly the fields a Reforger `game.mods[]` entry needs —
# `workshop_id`, `mod_guid`, `version` — added by T-271 in
# apps/website/api/migrations/0012_modpack_mods_workshop.sql, whose header says
# verbatim: "keep both so a future renderer (T-288) can choose".
#
# REJECTED — reading Postgres directly: this script is not a DB client (no psql, no
# DATABASE_URL in scripts/deploy/deploy.env.example), the database lives inside docker
# compose on the remote host, and hand-rolling the projection in bash would duplicate
# the null-tolerant COALESCE read in handlers/modpacks.rs `mod_cols!()`. The next
# migration would break the renderer silently. That is a second unconnected universe,
# not a fix.
# REJECTED — inventing a modpack file format of our own: that IS the defect this
# ticket exists to remove.
#
# ⚠ THE CREDENTIAL DOES NOT EXIST YET — see the report on T-288.
# `/modpacks/current` is gated by `AuthUser`, which is a **Bearer JWT** minted from a
# Discord login (apps/website/api/src/middleware/auth.rs). This script's only secret is
# TBD_GAME_SERVER_TOKEN, which is `SERVICE_TOKEN` (config.rs) and is checked by
# `ServiceAuth` against the **X-Service-Token** header — a different auth tier, and no
# ServiceAuth-guarded modpack read exists. So TBD_MODPACK_URL cannot be satisfied by
# anything the deploy host holds today; it is wired and fails closed, ready for the day
# a service-token modpack read (or a deploy JWT) ships.
#
#   TBD_MODPACK_JSON   path to a file holding a GET /modpacks/current response body.
#                      Works TODAY: save the response (or the DB row shaped like it)
#                      next to deploy.env. This is the supported path right now.
#   TBD_MODPACK_URL    fetch that same document over HTTP. Needs TBD_MODPACK_TOKEN.
#   TBD_MODPACK_TOKEN  Bearer JWT for the above. No such credential exists yet.
#
# Neither set → LEGACY single-mod render from TBD_WORKSHOP_MOD_ID (pre-T-288 behaviour,
# kept so existing deploys do not break), but it now goes through the SAME renderer and
# the SAME validator, so there is exactly one place that can emit game.mods[].
: "${TBD_MODPACK_JSON:=}"
: "${TBD_MODPACK_URL:=}"
: "${TBD_MODPACK_TOKEN:=}"
: "${TBD_WORKSHOP_MOD_NAME:=TBD_Framework}"   # legacy path's game.mods[0].name

if [[ "$TBD_REMOTE_DIR" == *prairielearn* ]]; then
  echo "Refusing to deploy: TBD_REMOTE_DIR must not be under prairielearn/" >&2
  exit 1
fi

case "$TBD_SERVER_MODE" in
  addons) ;;
  config)
    # T-288: TBD_WORKSHOP_MOD_ID is the LEGACY single-mod source and is only required
    # when no modpack document is configured — a modpack carries its own workshop ids.
    if [ -z "$TBD_WORKSHOP_MOD_ID" ] && [ -z "$TBD_MODPACK_JSON" ] && [ -z "$TBD_MODPACK_URL" ]; then
      echo "TBD_SERVER_MODE=config requires TBD_WORKSHOP_MOD_ID (publish tbd-framework" >&2
      echo "to the Workshop first, then set its modId in deploy.env), or a modpack" >&2
      echo "source: TBD_MODPACK_JSON=<file> / TBD_MODPACK_URL=<url> (T-288)." >&2
      exit 1
    fi
    if [ "$TBD_A2S_PORT" = "$TBD_GAME_PORT" ]; then
      echo "TBD_A2S_PORT must differ from TBD_GAME_PORT (a2s/game can't share a UDP port)." >&2
      exit 1
    fi
    ;;
  *)
    echo "Invalid TBD_SERVER_MODE='$TBD_SERVER_MODE' (expected: addons | config)" >&2
    exit 1
    ;;
esac

# ── T-288 render half ────────────────────────────────────────────────────────
#
# The render is now a LOCAL, pure function that writes a file. The push is a
# separate step that copies that file. Before T-288 the two were fused into one
# `ssh_cmd "cat > remote" <<EOF` heredoc, which meant the only way to see what this
# script produces was to deploy it to a live server — so nothing ever checked it.
#
# json/parse work is python3 because `jq` is NOT installed here (measured) and
# hand-rolled JSON in bash silently emits invalid documents. python3 is used by
# scripts/platform/{wave,preflight}.sh and scripts/mod/verify-t438-*.sh already.
# Required for config mode only; `addons` mode renders no config and is unaffected.
require_python3() {
  if ! command -v python3 >/dev/null 2>&1; then
    echo "FAIL: python3 not found, and TBD_SERVER_MODE=config needs it to render and" >&2
    echo "      validate game.mods[]. Refusing to emit an unvalidated server config —" >&2
    echo "      a config nobody checked is how a wrong mod list reaches a live server." >&2
    exit 1
  fi
}

# Resolve the modpack document (GET /modpacks/current shape) into $1.
resolve_modpack_doc() {
  local out="$1"
  if [ -n "$TBD_MODPACK_JSON" ]; then
    if [ ! -f "$TBD_MODPACK_JSON" ]; then
      echo "FAIL: TBD_MODPACK_JSON=$TBD_MODPACK_JSON does not exist." >&2
      exit 1
    fi
    cat "$TBD_MODPACK_JSON" > "$out"
    echo "  modpack source: file $TBD_MODPACK_JSON"
  elif [ -n "$TBD_MODPACK_URL" ]; then
    if [ -z "$TBD_MODPACK_TOKEN" ]; then
      echo "FAIL: TBD_MODPACK_URL is set but TBD_MODPACK_TOKEN is empty." >&2
      echo "      GET /api/v1/modpacks/current is gated by AuthUser (Bearer JWT," >&2
      echo "      apps/website/api/src/middleware/auth.rs). TBD_GAME_SERVER_TOKEN is the" >&2
      echo "      SERVICE_TOKEN checked on the X-Service-Token header by ServiceAuth and" >&2
      echo "      will NOT authenticate this route. See T-288." >&2
      exit 1
    fi
    local code
    code="$(curl -sS -o "$out" -w '%{http_code}' \
      -H "Authorization: Bearer $TBD_MODPACK_TOKEN" "$TBD_MODPACK_URL")" || {
      echo "FAIL: could not reach $TBD_MODPACK_URL" >&2
      exit 1
    }
    if [ "$code" != "200" ]; then
      echo "FAIL: $TBD_MODPACK_URL returned HTTP $code (expected 200)." >&2
      echo "      401/403 means the credential tier is wrong — see T-288." >&2
      exit 1
    fi
    echo "  modpack source: $TBD_MODPACK_URL (HTTP 200)"
  else
    # LEGACY: synthesize the same document shape from the env var so there is one
    # renderer and one validator, not two divergent code paths.
    TBD_WORKSHOP_MOD_ID="$TBD_WORKSHOP_MOD_ID" \
    TBD_WORKSHOP_MOD_NAME="$TBD_WORKSHOP_MOD_NAME" \
    MODPACK_OUT="$out" python3 - <<'PY'
import json, os
json.dump({
    "name": "(legacy TBD_WORKSHOP_MOD_ID env, not a database modpack)",
    "version": "",
    "mods": [{
        "name": os.environ["TBD_WORKSHOP_MOD_NAME"],
        "workshop_id": os.environ["TBD_WORKSHOP_MOD_ID"],
        "version": "",
    }],
}, open(os.environ["MODPACK_OUT"], "w", encoding="utf-8"))
PY
    echo "  modpack source: LEGACY env TBD_WORKSHOP_MOD_ID (no modpack configured)"
  fi
}

# Print the `game.mods[]` array (JSON) for the modpack document in $1, or fail closed.
modpack_mods_json() {
  MODPACK_DOC="$1" MODPACK_SRC="${2:-$1}" python3 - <<'PY'
import json, os, sys

path = os.environ["MODPACK_DOC"]
src = os.environ.get("MODPACK_SRC") or path
try:
    with open(path, encoding="utf-8") as fh:
        doc = json.load(fh)
except json.JSONDecodeError as e:
    sys.exit("FAIL: modpack document is not valid JSON (%s): %s" % (src, e))

if not isinstance(doc, dict):
    sys.exit("FAIL: modpack document must be a JSON object, got %s" % type(doc).__name__)

mods = doc.get("mods")
if mods is None:
    sys.exit(
        "FAIL: modpack document has no `mods` key. Expected the body of\n"
        "      GET /api/v1/modpacks/current (ModpackDto = modpack fields + mods[])."
    )
if not isinstance(mods, list):
    sys.exit("FAIL: `mods` must be an array, got %s" % type(mods).__name__)
if not mods:
    sys.exit(
        "FAIL: modpack %r has zero mods. Rendering game.mods[] as [] would start a\n"
        "      server with no content and silently disagree with the website."
        % (doc.get("name") or "<unnamed>")
    )

out, seen = [], {}
for i, m in enumerate(mods):
    if not isinstance(m, dict):
        sys.exit("FAIL: mods[%d] is not an object" % i)
    name = str(m.get("name") or "").strip()
    wid = str(m.get("workshop_id") or "").strip()
    ver = str(m.get("version") or "").strip()
    if not name:
        sys.exit("FAIL: mods[%d].name is empty" % i)
    if not wid:
        sys.exit(
            "FAIL: mods[%d] (%r) has an empty workshop_id.\n"
            "      Reforger game.mods[].modId IS the Workshop id; an empty one renders\n"
            '      "modId": "" and the server rejects the config. Populate\n'
            "      modpack_mods.workshop_id (migration 0012_modpack_mods_workshop.sql)."
            % (i, name)
        )
    if wid in seen:
        sys.exit("FAIL: mods[%d] (%r) repeats modId %s, already used by %r"
                 % (i, name, wid, seen[wid]))
    seen[wid] = name
    entry = {"modId": wid, "name": name}
    if ver:
        entry["version"] = ver
    out.append(entry)

text = json.dumps(out, indent=2)
print("\n".join(ln if i == 0 else "    " + ln for i, ln in enumerate(text.splitlines())))
PY
}

# Structural check of a rendered server config. NOT eyeballing: re-parses the file and
# pins the invariants the Reforger server enforces (and the a2s/game port rule this
# script already documents at TBD_A2S_PORT).
validate_server_config() {
  SERVER_CONFIG="$1" python3 - <<'PY'
import json, os, sys

path = os.environ["SERVER_CONFIG"]
try:
    with open(path, encoding="utf-8") as fh:
        doc = json.load(fh)
except json.JSONDecodeError as e:
    sys.exit("FAIL: rendered server config is not valid JSON (%s): %s" % (path, e))

errs = []
for key in ("bindAddress", "bindPort", "publicAddress", "publicPort", "a2s", "game", "operating"):
    if key not in doc:
        errs.append("missing top-level key %r" % key)

game = doc.get("game") if isinstance(doc.get("game"), dict) else {}
for key in ("name", "passwordAdmin", "admins", "scenarioId", "maxPlayers", "mods"):
    if key not in game:
        errs.append("missing game.%s" % key)

a2s = doc.get("a2s") if isinstance(doc.get("a2s"), dict) else {}
if a2s.get("port") is not None and a2s.get("port") == doc.get("bindPort"):
    errs.append("a2s.port == bindPort (%r) — replication cannot start" % doc.get("bindPort"))

mods = game.get("mods")
if not isinstance(mods, list) or not mods:
    errs.append("game.mods[] must be a non-empty array")
    mods = []
for i, m in enumerate(mods):
    if not isinstance(m, dict):
        errs.append("game.mods[%d] is not an object" % i)
        continue
    if not str(m.get("modId") or "").strip():
        errs.append("game.mods[%d].modId is empty" % i)
    if not str(m.get("name") or "").strip():
        errs.append("game.mods[%d].name is empty" % i)

if errs:
    sys.exit("FAIL: rendered server config is invalid:\n      " + "\n      ".join(errs))

print("  config VALID: %d mod(s) -> %s" % (
    len(mods), ", ".join("%s=%s" % (m["name"], m["modId"]) for m in mods)))
PY
}

# Render the complete server config to the LOCAL path $1.
render_server_config() {
  local out="$1"
  require_python3

  local doc
  doc="$(mktemp -t tbd-modpack.XXXXXX.json)"
  # shellcheck disable=SC2064
  trap "rm -f '$doc'" RETURN
  resolve_modpack_doc "$doc"

  local mods_json src_label
  src_label="${TBD_MODPACK_JSON:-${TBD_MODPACK_URL:-LEGACY TBD_WORKSHOP_MOD_ID}}"
  # exits non-zero (set -e) on any fail-closed case
  mods_json="$(modpack_mods_json "$doc" "$src_label")"

  # Build a JSON array of admin identityIds from the comma-separated env var.
  local admins_json="" _aid
  if [ -n "$TBD_ADMIN_IDENTITY_IDS" ]; then
    local -a _admin_ids
    IFS=',' read -ra _admin_ids <<< "$TBD_ADMIN_IDENTITY_IDS"
    for _aid in "${_admin_ids[@]}"; do
      _aid="$(echo "$_aid" | xargs)"
      [ -n "$_aid" ] && admins_json="${admins_json:+$admins_json, }\"$_aid\""
    done
  fi

  cat > "$out" <<EOF
{
  "bindAddress": "0.0.0.0",
  "bindPort": ${TBD_GAME_PORT},
  "publicAddress": "${TBD_PUBLIC_ADDRESS}",
  "publicPort": ${TBD_GAME_PORT},
  "a2s": { "address": "0.0.0.0", "port": ${TBD_A2S_PORT} },
  "game": {
    "name": "${TBD_SERVER_NAME}",
    "password": "",
    "passwordAdmin": "${TBD_ADMIN_PASSWORD}",
    "admins": [${admins_json}],
    "scenarioId": "${TBD_SCENARIO}",
    "maxPlayers": ${TBD_MAX_PLAYERS},
    "visible": true,
    "crossPlatform": false,
    "gameProperties": {
      "battlEye": false,
      "disableThirdPerson": false,
      "fastValidation": false,
      "VONDisableUI": false,
      "VONDisableDirectSpeechUI": false
    },
    "mods": ${mods_json}
  },
  "operating": { "lobbyPlayerSynchronise": true }
}
EOF

  validate_server_config "$out"
}

# --render-only: render locally, validate, exit. Reached BEFORE any cargo/rsync/ssh,
# so it can never touch a server.
if [ -n "$RENDER_ONLY_OUT" ]; then
  if [ "$TBD_SERVER_MODE" != "config" ]; then
    echo "--render-only requires TBD_SERVER_MODE=config (addons mode renders no config)." >&2
    exit 2
  fi
  echo "==> render server config (local only, no deploy) -> $RENDER_ONLY_OUT"
  render_server_config "$RENDER_ONLY_OUT"
  exit 0
fi

SSH_BASE=(ssh -o StrictHostKeyChecking=no)
if [ -n "${TBD_SSH_PASS:-}" ]; then
  SSH_BASE=(sshpass -p "$TBD_SSH_PASS" ssh -o StrictHostKeyChecking=no)
elif [ -n "${TBD_SSH_IDENTITY_FILE:-}" ]; then
  SSH_BASE=(ssh -i "$TBD_SSH_IDENTITY_FILE" -o StrictHostKeyChecking=no)
fi

run() {
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "[dry-run] $*"
  else
    "$@"
  fi
}

ssh_cmd() {
  run "${SSH_BASE[@]}" "$TBD_SSH_HOST" "$@"
}

rsync_to_remote() {
  local -a rsync_ssh
  if [ -n "${TBD_SSH_PASS:-}" ]; then
    rsync_ssh=(-e "sshpass -p $TBD_SSH_PASS ssh -o StrictHostKeyChecking=no")
  elif [ -n "${TBD_SSH_IDENTITY_FILE:-}" ]; then
    rsync_ssh=(-e "ssh -i $TBD_SSH_IDENTITY_FILE -o StrictHostKeyChecking=no")
  else
    rsync_ssh=(-e "ssh -o StrictHostKeyChecking=no")
  fi
  run rsync "${rsync_ssh[@]}" "$@"
}

echo "==> V1 validate mission JSON"
if [ "$DRY_RUN" -eq 0 ]; then
  cargo run -q -p xtask -- schema validate-file \
    "$SCHEMA/golden-missions/${TBD_MISSION_ID}.json"
fi

echo "==> rsync to $TBD_REMOTE_DIR"
# T-181.52 — EXCLUDE EVERY ORACLE LANE, not just CRF. These are read-only reference trees; the
# server only ever runs apps/mod/tbd-framework (see the addon symlink below), so shipping them is
# pure licence exposure for zero benefit. crf_framework was already excluded, but vanilla_reference
# and playable_selector were NOT — and in the MAIN checkout (which is what deploys) they are real
# directories, not the worktree symlinks, so ~30 MB of carved Bohemia game source was being rsynced
# to staging on every deploy. playable_selector has NO LICENCE AT ALL, so copying it to a server is
# redistribution we have no permission for. Anyone adding a fourth oracle lane adds it here too.
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] rsync -avz --delete ... $TBD_SSH_HOST:$TBD_REMOTE_DIR/"
else
  rsync_to_remote -avz --delete \
    --exclude=.git/ \
    --exclude=apps/mod/crf_framework/ \
    --exclude=apps/mod/vanilla_reference/ \
    --exclude=apps/mod/playable_selector/ \
    --exclude=apps/mod/Tbd_framework/ \
    --exclude=apps/mod/.local-test-profile/ \
    --exclude='**/node_modules/' \
    --exclude=apps/website/api/.tools/ \
    --exclude=apps/website/api/.env \
    --exclude=apps/mod/tbd-framework/Scripts/WorkbenchGame/ \
    --exclude=scripts/deploy/deploy.env \
    "$MONO_ROOT/" "$TBD_SSH_HOST:$TBD_REMOTE_DIR/"
fi

echo "==> remote profile + addon symlink"
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] setup-server-profile + patch TBD_BackendConfig.json"
else
  ssh_cmd bash -s <<EOF
set -euo pipefail
mkdir -p "$TBD_ADDONS_STAGING" "$TBD_PROFILE_DIR"
ln -sfn "$TBD_REMOTE_DIR/apps/mod/tbd-framework" "$TBD_ADDONS_STAGING/tbd-framework"
export GAME_SERVER_TOKEN='$TBD_GAME_SERVER_TOKEN'
bash "$TBD_REMOTE_DIR/scripts/mod/setup-server-profile.sh" "$TBD_PROFILE_DIR"
CFG="$TBD_PROFILE_DIR/profile/TBD_BackendConfig.json"
sed -i "s|replace-with-GAME_SERVER_TOKENS-value|$TBD_GAME_SERVER_TOKEN|g" "\$CFG"
sed -i 's|"backendUrl": "[^"]*"|"backendUrl": "$TBD_BACKEND_URL"|' "\$CFG"
sed -i 's|"missionId": "[^"]*"|"missionId": "$TBD_MISSION_ID"|' "\$CFG"
sed -i 's|"eventId": "[^"]*"|"eventId": "$TBD_EVENT_ID"|' "\$CFG"
EOF
fi

echo "==> docker compose (API + Postgres)"
# T-438: compose file lives at apps/website/docker-compose.staging.yml (T-251),
# not under apps/website/api/. Match scripts/deploy/deploy-website.sh.
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] cd \$TBD_REMOTE_DIR && docker compose -f apps/website/docker-compose.staging.yml up -d --build"
else
  ssh_cmd "cd '$TBD_REMOTE_DIR' && docker compose -f apps/website/docker-compose.staging.yml up -d --build"
fi

# V2–V4 hit the game-server REST routes (/api/missions/:id/compiled, /api/game/.../roster).
# Those existed only in the Phase-0 REST spike backend, since removed — the current backend
# serves /api/v1 only, so these curls 404 and would abort the deploy. BLOCKED on T-092
# (docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md). Skipped by
# default until T-092 ships; set TBD_RUN_T092_SMOKE=1 to force-run the gate.
echo "==> API smoke (V2–V4)"
if [ "${TBD_RUN_T092_SMOKE:-0}" != "1" ]; then
  echo "[SKIP] V2–V4 API smoke — routes BLOCKED on T-092 (not in current backend; would 404)."
  echo "       Set TBD_RUN_T092_SMOKE=1 to force once T-092 ships. See docs/mod/STAGING-SERVER.md."
elif [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] curl mission + roster + 401 on server localhost"
else
  ssh_cmd bash -s <<EOF
set -euo pipefail
TOKEN='$TBD_GAME_SERVER_TOKEN'
MID='$TBD_MISSION_ID'
EID='$TBD_EVENT_ID'
code=\$(curl -sS -o /tmp/tbd-mission.json -w '%{http_code}' -H "Authorization: Bearer \$TOKEN" \\
  "http://127.0.0.1:8080/api/missions/\$MID/compiled")
echo "V2 mission compiled: HTTP \$code"
[ "\$code" = "200" ] || exit 1
code=\$(curl -sS -o /tmp/tbd-roster.json -w '%{http_code}' -H "Authorization: Bearer \$TOKEN" \\
  "http://127.0.0.1:8080/api/game/events/\$EID/roster")
echo "V3 roster: HTTP \$code"
[ "\$code" = "200" ] || exit 1
code=\$(curl -sS -o /dev/null -w '%{http_code}' "http://127.0.0.1:8080/api/missions/\$MID/compiled")
echo "V4 unauth: HTTP \$code"
[ "\$code" = "401" ] || exit 1
EOF
fi

# Build ExecStart per mode (NOTE: -config is mutually exclusive with -addons).
if [ "$TBD_SERVER_MODE" = "config" ]; then
  EXECSTART="${TBD_SERVER_DIR}/ArmaReforgerServer -profile ${TBD_PROFILE_DIR} -config ${TBD_SERVER_CONFIG_REMOTE} -maxFPS 60 -logStats 30000 -nothrow"
else
  EXECSTART="${TBD_SERVER_DIR}/ArmaReforgerServer -profile ${TBD_PROFILE_DIR} -addonsDir ${TBD_ADDONS_STAGING} -addons ${TBD_ADDON_GUID} -server \"${TBD_SCENARIO}\" -bindIP 0.0.0.0 -bindPort ${TBD_GAME_PORT} -a2sPort ${TBD_A2S_PORT} -maxFPS 60 -logStats 30000 -nothrow"
fi

echo "==> systemd user service + restart game server (mode: $TBD_SERVER_MODE)"
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] mode=$TBD_SERVER_MODE"
  if [ "$TBD_SERVER_MODE" = "config" ]; then
    # T-288: name the ACTUAL mod-list source. "modId=$TBD_WORKSHOP_MOD_ID" was the only
    # thing this ever printed, which read as "the mod list is fine" on a run whose mod
    # list came from nowhere near the modpack the operator had authored.
    if [ -n "$TBD_MODPACK_JSON" ]; then
      _mod_src="modpack file $TBD_MODPACK_JSON"
    elif [ -n "$TBD_MODPACK_URL" ]; then
      _mod_src="modpack API $TBD_MODPACK_URL"
    else
      _mod_src="LEGACY single mod TBD_WORKSHOP_MOD_ID=$TBD_WORKSHOP_MOD_ID (no modpack configured)"
    fi
    echo "[dry-run] render server config -> $TBD_SERVER_CONFIG_REMOTE"
    echo "[dry-run]   game.mods[] from: $_mod_src"
    echo "[dry-run]   preview the exact bytes with: --render-only <path>"
  fi
  echo "[dry-run] ExecStart=$EXECSTART"
  echo "[dry-run] install tbd-reforger.service and restart"
else
  # In config mode, render the server config JSON LOCALLY, validate it, and only then
  # push it (registers the backend room; the Workshop mods are downloaded from
  # game.mods[]). T-288 split render from push: an invalid or empty mod list now fails
  # here, on the dev machine, instead of landing on the server and failing at boot.
  if [ "$TBD_SERVER_MODE" = "config" ]; then
    TBD_SERVER_CONFIG_LOCAL="$(mktemp -t tbd-server.config.XXXXXX.json)"
    render_server_config "$TBD_SERVER_CONFIG_LOCAL"
    ssh_cmd "cat > '$TBD_SERVER_CONFIG_REMOTE'" < "$TBD_SERVER_CONFIG_LOCAL"
    rm -f "$TBD_SERVER_CONFIG_LOCAL"
  fi

  ssh_cmd bash -s <<EOF
set -euo pipefail
UNIT="\$HOME/.config/systemd/user/tbd-reforger.service"
mkdir -p "\$HOME/.config/systemd/user"
cat > "\$UNIT" <<'UNITEOF'
[Unit]
Description=TBD Arma Reforger dedicated server (TBD_Dev_POC, mode=${TBD_SERVER_MODE})
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=${TBD_SERVER_DIR}
ExecStart=${EXECSTART}
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
UNITEOF
systemctl --user daemon-reload
systemctl --user enable tbd-reforger.service 2>/dev/null || true
systemctl --user restart tbd-reforger.service 2>/dev/null || systemctl --user start tbd-reforger.service
EOF
  sleep 8
fi

echo "==> V6 remote log grep"
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] bash scripts/mod/remote-log-grep.sh"
  exit 0
fi

bash "$MOD_SCRIPTS/remote-log-grep.sh"
