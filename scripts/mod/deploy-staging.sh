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
VERIFY_BOOT_LOG=""
VERIFY_BOOT_SELFTEST=0

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
    --verify-boot)
      # T-607: run the boot verdict over a console.log you already have — no ssh, no
      # deploy.env, no staging host. Same split --render-only made for the server config:
      # a check that only runs mid-deploy is a check nobody runs.
      shift
      VERIFY_BOOT_LOG="${1:-}"
      if [ -z "$VERIFY_BOOT_LOG" ]; then
        echo "--verify-boot requires a path to a console.log" >&2
        exit 2
      fi
      ;;
    --verify-boot-selftest)
      # T-607: prove the boot verdict can FAIL. A gate never observed failing is not a gate.
      VERIFY_BOOT_SELFTEST=1
      ;;
    -h|--help)
      echo "Usage: deploy-staging.sh [--dry-run] [--render-only <path>]"
      echo "                         [--render-agent <dir>] [--agent-selftest <dir>]"
      echo "                         [--verify-boot <console.log>] [--verify-boot-selftest]"
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

  # ── The socket half ────────────────────────────────────────────────────────
  #
  # Everything above drives the agent as a stdin/stdout filter, which proves the LOGIC and
  # nothing about the CHANNEL. This block opens a real AF_UNIX socket, activates the agent
  # through it exactly as `Accept=yes` + `StandardInput=socket` will on the host, and reads
  # the reply back off the wire. Without it, "the agent works" would be a claim about a
  # program nobody ever connected to.
  #
  # FAIL CLOSED on a missing tool rather than skipping. systemd-socket-activate ships in
  # the base systemd package, and this script installs systemd units for a living — an
  # environment without it cannot validate this artefact, and should say so rather than
  # print a green line about a check that did not execute.
  if ! command -v systemd-socket-activate >/dev/null 2>&1; then
    echo "  FAIL  socket round-trip — systemd-socket-activate not found."
    echo "        Refusing to report the channel OK without ever opening it."
    fail=$((fail + 1))
  elif ! command -v python3 >/dev/null 2>&1; then
    echo "  FAIL  socket round-trip — python3 (the test client) not found."
    fail=$((fail + 1))
  elif ! command -v setsid >/dev/null 2>&1; then
    echo "  FAIL  socket round-trip — setsid not found (see the note below on why it is required)."
    fail=$((fail + 1))
  else
    # name | STUB_ACTIVE | request | expected result | expected state
    local -a sock_cases=(
      "socket round-trip, healthy unit|active|restart|accepted|active"
      "socket round-trip, systemctl exits 0 over a DEAD unit|failed|restart|rejected|failed"
    )
    local sc sock_name sock_active sock_req sock_want_r sock_want_s sock_path reply
    local i=0
    for sc in "${sock_cases[@]}"; do
      IFS='|' read -r sock_name sock_active sock_req sock_want_r sock_want_s <<< "$sc"
      i=$((i + 1))
      sock_path="$d/agent-$i.sock"
      rm -f "$sock_path"
      # setsid is LOAD-BEARING, not tidiness. `systemd-socket-activate` re-broadcasts a
      # received SIGTERM to its whole process group, so a plain `kill $!` from here kills
      # THIS SCRIPT TOO — measured: the selftest exited 143 with the socket cases never
      # reported. Its own session means the teardown below can only reach the listener.
      setsid systemd-socket-activate --listen="$sock_path" --accept --inetd \
        -E "PATH=$d/bin:$PATH" -E "TBD_AGENT_UNIT=$TBD_AGENT_UNIT" \
        -E "STUB_LOAD=loaded" -E "STUB_ACTIVE=$sock_active" -E "STUB_VERB_RC=0" \
        -E "TBD_AGENT_DWELL_S=0" \
        -- bash "$d/tbd-reforger-agent.sh" >"$d/activate-$i.log" 2>&1 &
      local sa_pid=$!
      # Wait for the listener rather than sleeping a guess.
      local waited=0
      while [ ! -S "$sock_path" ] && [ "$waited" -lt 50 ]; do
        sleep 0.1
        waited=$((waited + 1))
      done
      reply="$(SOCK="$sock_path" REQ="$sock_req" python3 -c '
import os, socket, sys
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(30)
try:
    s.connect(os.environ["SOCK"])
    s.sendall(os.environ["REQ"].encode() + b"\n")
    s.shutdown(socket.SHUT_WR)
    sys.stdout.write(s.recv(4096).decode().strip())
except OSError as e:
    sys.stdout.write("CLIENT-ERROR %s" % e)
' 2>/dev/null)"
      # `|| true` on BOTH: `wait` returns the job's status, which for a SIGTERMed listener is
      # 143, and under `set -e` that aborts the selftest before it can report anything. Also
      # measured — the run exited 143 with zero socket lines printed.
      kill "$sa_pid" 2>/dev/null || true
      wait "$sa_pid" 2>/dev/null || true
      got_result="$(printf '%s' "$reply" | sed -n 's/.*"result":"\([a-z]*\)".*/\1/p')"
      got_state="$(printf '%s' "$reply" | sed -n 's/.*"state":"\([a-z-]*\)".*/\1/p')"
      if [ "$got_result" = "$sock_want_r" ] && [ "$got_state" = "$sock_want_s" ]; then
        echo "  PASS  $sock_name -> $got_result/$got_state"
        pass=$((pass + 1))
      else
        echo "  FAIL  $sock_name"
        echo "        want result=$sock_want_r state=$sock_want_s"
        echo "        got  result=$got_result state=$got_state"
        echo "        raw  $reply"
        fail=$((fail + 1))
      fi
    done
  fi

  # The rendered units must be units systemd itself accepts, not just files that grep right.
  if command -v systemd-analyze >/dev/null 2>&1; then
    if systemd-analyze verify --user "$d/tbd-reforger-agent.socket" >"$d/analyze.log" 2>&1; then
      echo "  PASS  systemd-analyze verify accepts the rendered socket unit"
      pass=$((pass + 1))
    else
      echo "  FAIL  systemd-analyze rejected the rendered socket unit:"
      sed 's/^/        /' "$d/analyze.log"
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

# ── WHAT THE API SLICE MUST BUILD ────────────────────────────────────────────
#
# apps/website/api/** is NOT this slice's to touch, and no sibling owns it this wave. The
# host half above is complete and proven; the API half is mechanical from here.
#
# 1. CONFIG — one new var in apps/website/api/src/config.rs (which today declares 18 and
#    not one that names a game host):
#       game_agent_socket: env::var("GAME_AGENT_SOCKET").unwrap_or_default()
#    Empty = no transport, and `send_rcon` keeps answering 503. Fail closed, like
#    TBD_MODPACK_URL does above. Populate it in the API's systemd unit
#    (docs/website/HOME_SERVER.md:282) as %t/tbd-reforger-agent.sock.
#
# 2. CLIENT — new apps/website/api/src/services/game_agent.rs. No new dependency: tokio is
#    already in the tree and `tokio::net::UnixStream` is all this needs.
#       pub enum AgentAction { Status, Start, Stop, Restart }   // Display -> the wire verb
#       #[derive(Deserialize)] pub struct AgentReply {
#           pub ok: bool, pub action: String, pub result: AgentResult,
#           pub state: String, pub detail: String }
#       #[derive(Deserialize)] #[serde(rename_all="lowercase")]
#       pub enum AgentResult { Accepted, Rejected, Unreachable }
#       pub async fn send(sock: &Path, a: AgentAction) -> anyhow::Result<AgentReply>
#    Body: connect, write "<verb>\n", read exactly one line, serde_json::from_str.
#    TIMEOUT MUST EXCEED THE DWELL — the agent sleeps TBD_AGENT_DWELL_S (default 8) before
#    answering start/restart, on purpose. Use 20s. A timeout shorter than the dwell would
#    turn every honest slow answer into a false "unreachable".
#
# 3. HANDLER — apps/website/api/src/handlers/admin.rs `send_rcon` (currently ends in the
#    unconditional `Err(ApiError::with_details(SERVICE_UNAVAILABLE, RCON_NO_TRANSPORT, …))`
#    at :628). Map the validated RconCommand, then map the reply — and note the mapping is
#    three-way, because that is the delivery result T-269 asked for:
#       RconCommand::Restart          -> AgentAction::Restart
#       RconCommand::Kick / ChangeMap / Custom -> STILL 503, unchanged (see §SCOPE GAP)
#       AgentResult::Accepted   -> 202 {"accepted":true,"delivered":true,"state":<state>}
#       AgentResult::Rejected   -> 409 — the agent ran it and the unit did NOT get there
#                                  (the a2sPort-clash case; `state` says which)
#       AgentResult::Unreachable-> 503 RCON_NO_TRANSPORT (socket absent / unit not installed)
#       transport error/timeout -> 503, same shape
#    THE AUDIT ROW MUST RECORD THE OUTCOME, NOT THE ATTEMPT — that is the specific defect
#    T-269 called out. Write it AFTER the agent answers, with severity Info on Accepted and
#    Warn otherwise, and put the observed `state` in the detail. A row saying "attempted"
#    when the thing succeeded is the same class of lie as one saying "issued" when it did not.
#
# 4. ADDRESSING — for THIS deployment nothing is needed in the `servers` table: one host,
#    one socket, path from config. T-269 asked for a migration because it assumed a network
#    hop; across a same-uid socket the OS is the credential (see the header). The migration
#    becomes REQUIRED the moment a second game host exists, and then it is:
#       ALTER TABLE servers ADD COLUMN agent_socket text;   -- local socket path, or
#       ALTER TABLE servers ADD COLUMN agent_endpoint text; -- host:port for a remote agent
#    plus a real credential column for the remote case, because the OS stops vouching for
#    the peer the moment the channel leaves the box. `send_rcon` already loads the `servers`
#    row (`SELECT name FROM servers WHERE id = $1`, admin.rs:601) — widen that select.
#
# ── SCOPE GAP, DECIDED ───────────────────────────────────────────────────────
#
# T-269 asked this ticket to decide whether change_map/custom need a fifth scope. They do,
# and it is a DIFFERENT TICKET, because they are not process control:
#
# * `restart` / `start` / `stop` / `status` are the unit's lifecycle. The agent covers them
#   completely and safely, and only these four are reachable over the socket.
# * `change_map` and `custom` need a live admin channel INTO a running server. Nothing in
#   this repo has one. Adding it means either enabling Reforger RCON (a new port, an admin
#   password in server.config.json, and a protocol we cannot exercise here) or a mod-side
#   command sink (a route the mod polls + a handler in tbd-framework). Either is strictly
#   larger than this ticket and must not be smuggled into the agent — the agent's safety
#   argument rests entirely on it accepting no free text.
# * `kick` CANNOT BE BUILT AT ALL YET, for a reason upstream of transport: `RconInput` has
#   no player field (admin.rs:422-428 — action/map/command only), so
#   apps/website/frontend/src/server_control.rs:44 posts a bare {"action":"kick"} that names
#   nobody. (T-269's summary cites :43; re-measured on main it is :44.) That is a UI + model
#   gap, and it must be fixed before any transport question about kick is even meaningful.

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

# ── T-607: THE BOOT VERDICT ──────────────────────────────────────────────────
#
# WHAT WAS BROKEN. This script built two ExecStarts and neither gave a server that was
# both correct and joinable:
#
#   config mode   -config, no -addonsDir   registers a room, resolves the mod from the
#                                          WORKSHOP — not from the checkout it just rsynced
#   addons mode   -addonsDir + -addons     loads the checkout, registers NO backend room
#                 + -server
#
# The first is the expensive one and it is this program's signature defect wearing the
# engine's clothes: **staging was validating a build it did not deploy.** `tbd-framework` is
# published unlisted under the SAME id as the local gproj GUID (B2C3D4E5F6A78901), so
# `game.mods[]` is satisfiable from the Workshop and the engine quietly does that. The deploy
# rsyncs a checkout to the host, symlinks it into $TBD_ADDONS_STAGING (line ~1100), and then
# launches a server that never looks at it. Every "staging is green" verdict since the June
# publish was a true statement about the WRONG code.
#
# THE FIX is T-604's, not a new one: `-addonsDir <dir>` **plus** `-config <json>` does both at
# once. See scripts/mod/run-playtest-server.sh, whose header carries the measured boot. The
# 2026-06-14 "mutually exclusive" finding was measured on `-addons`, which really is fatal with
# `-config`; `-addonsDir` is a different flag and combines with it fine.
#
# WHY THE ASSERTION BELOW IS NOT OPTIONAL, AND WHY IT MATTERS MORE HERE THAN IN THE PLAYTEST.
# run-playtest-server.sh boots whatever is in your working tree, so a wrong answer there is
# merely confusing. Staging deploys a SPECIFIC checkout and its whole job is to report on that
# checkout, so "did the thing I deployed actually load" is the entire product. A green deploy
# over the Workshop copy is worse than a failed one.
#
# ⚠ THE FORMAT CHECK NO LONGER DISCRIMINATES HERE. `remote-log-grep.sh` separates builds with
# `grep -c '\[TBD\]\['` — stale Workshop 1.0.1 emits 0, any current build emits many. That was
# sound while the Workshop copy was June's. The operator re-published on 2026-07-31, so the
# Workshop now serves **1.0.2**, which is current-format: measured on a real `-config`-only boot
# (2026-08-01 00:12, profile pak 570,489 B) that log carries **154** `[TBD][` lines and would
# sail through the format threshold while running a pak the deploy never produced. The format
# check answers "is this build ancient", which is a different question from "is this build the
# one I just deployed". Only the gproj PATH answers the second, which is why it is asserted here
# and why this assertion cannot be replaced by a line count. Do not "simplify" it into one.
#
# Pure functions over a log FILE on purpose: the deploy half needs ssh and a live host, and a
# check that can only run during a real deploy is a check nobody runs. `--verify-boot` and
# `--verify-boot-selftest` exercise every line of this logic with no ssh, no deploy.env and no
# staging host, exactly as --render-only/--agent-selftest do for the other two artefacts.

# The addon GUID, read from the gproj rather than trusted from deploy.env. A literal drifts
# from the source silently; run-playtest-server.sh:566 and world-boot.sh:376 both read it.
read_addon_guid() {
  local gproj="$MONO_ROOT/apps/mod/tbd-framework/addon.gproj"
  [ -f "$gproj" ] || return 1
  grep -oE '^[[:space:]]*GUID[[:space:]]+"[0-9A-Fa-f]+"' "$gproj" | grep -oE '[0-9A-Fa-f]{8,}' | head -1
}

# THE HARD GATE. Did the addon we deployed win, or a packed copy from the Workshop?
#
# The discriminator is the gproj path the engine reports under `Loaded addons:` for OUR guid:
#
#   deployed checkout won   gproj: '<addonsDir>/tbd-framework/addon.gproj' guid: '<GUID>'
#   Workshop copy won       gproj: '<profile>/addons/TBDFramework_<GUID>/addon.gproj' guid: '<GUID>'
#
# Both are `guid: '<GUID>'` and both look healthy. Only the path differs, so only the path is
# checked. The last block wins: the engine prints `Loaded addons:` more than once per boot (once
# before and once after addon resolution — measured, 2 blocks on both a passing and a failing
# log) and the final one is the one that ran.
assert_local_addon_won() {
  local log="$1" guid="$2" addons_dir="$3"
  local want="$addons_dir/tbd-framework/addon.gproj"

  if [ ! -f "$log" ]; then
    echo "FAIL: boot log not found: $log" >&2
    echo "      The check did NOT run. This is not a pass." >&2
    return 1
  fi

  # -F: the path may contain regex metacharacters, and `guid: '...'` is a literal.
  local loaded
  loaded="$(grep -A8 'Loaded addons:' "$log" | grep -F "guid: '$guid'" | tail -1)"
  if [ -z "$loaded" ]; then
    echo "FAIL: the engine never reported loading addon $guid at all." >&2
    echo "      Neither copy won, so the mod is simply not running." >&2
    grep -nE "Loaded addons:|gproj:" "$log" | head -20 >&2
    return 1
  fi

  case "$loaded" in
    *"$want"*)
      echo "  PASS  deployed checkout won: $want"
      return 0
      ;;
  esac

  echo "FAIL: STAGING IS VALIDATING A BUILD IT DID NOT DEPLOY." >&2
  echo "  loaded: $(printf '%s' "$loaded" | sed 's/^[[:space:]]*//')" >&2
  echo "  wanted: $want" >&2
  echo "" >&2
  echo "  tbd-framework is published to the Workshop unlisted under the SAME id as the local" >&2
  echo "  gproj GUID, so the engine can satisfy game.mods[] without ever reading the checkout" >&2
  echo "  this deploy just rsynced. Every log line after this one would be a true statement" >&2
  echo "  about the wrong code." >&2
  echo "" >&2
  echo "  Cause is almost always a missing -addonsDir on the ExecStart. Check the unit:" >&2
  echo "      systemctl --user cat tbd-reforger.service | grep ExecStart" >&2
  echo "  It must carry BOTH -addonsDir and -config (T-604)." >&2
  return 1
}

# The other half of the ticket: addons mode loads the right code and registers no room, so a
# server can be running the correct build and still be unjoinable. Assert the room, by the
# engine's own line, not by inference from a healthy-looking log.
assert_room_registered() {
  local log="$1"
  if [ ! -f "$log" ]; then
    echo "FAIL: boot log not found: $log" >&2
    return 1
  fi
  local reg
  reg="$(grep -F 'Server registered with address:' "$log" | tail -1)"
  if [ -z "$reg" ]; then
    echo "FAIL: no backend room registered — the server is NOT joinable." >&2
    echo "      Zero 'Server registered with address:' lines in $log." >&2
    echo "      A healthy log is not a joinable server: -addonsDir + -addons + -server reaches" >&2
    echo "      LOBBY with the mod loaded and never registers a room. Direct Join answers" >&2
    echo "      'No server found'. Joinable needs -config, alongside -addonsDir (T-604)." >&2
    return 1
  fi
  echo "  PASS  backend room registered: $(printf '%s' "$reg" | sed 's/.*Server registered/Server registered/')"
  return 0
}

# `#tbd` resolves admins from vanilla's SCR_PlayerListedAdminManagerComponent, which is
# populated ONLY from game.admins[] in the server config — TBD_AdminService.IsAdmin() defers to
# it. addons mode has no config at all, so it can never have an admin; that is the second half
# of "the two modes break different halves of the acceptance criteria". passwordAdmin is a
# DIFFERENT mechanism and does not feed that list.
#
# What a log CAN prove is that the engine accepted the config carrying them. Whether a given id
# maps to the human who connects is only observable when they connect, and this says so rather
# than implying otherwise.
assert_admins_configured() {
  local log="$1" want_count="$2"
  if [ ! -f "$log" ]; then
    echo "FAIL: boot log not found: $log" >&2
    return 1
  fi
  if ! grep -qF 'Server config loaded.' "$log"; then
    echo "FAIL: the engine never loaded a server config — game.admins[] cannot exist." >&2
    echo "      '#tbd' will answer 'TBD: admin only.' for everyone, whatever deploy.env says." >&2
    return 1
  fi
  if ! grep -qF 'JSON is Valid' "$log"; then
    echo "FAIL: the engine did not report the server config as schema-valid." >&2
    grep -nE 'JSON Schema Validation|RegEx Pattern|errors in server config' "$log" | head -10 >&2
    return 1
  fi
  if [ "$want_count" -eq 0 ]; then
    echo "  WARN  config accepted, but game.admins[] is EMPTY (TBD_ADMIN_IDENTITY_IDS unset)."
    echo "        Every '#tbd' command will answer 'TBD: admin only.' Set it in deploy.env."
    return 0
  fi
  echo "  PASS  server config accepted by the engine, carrying $want_count admin id(s)"
  echo "        (that the ENGINE took them; whether an id is the human who connects is only"
  echo "         observable when they connect — check '#tbd' in chat)"
  return 0
}

# The whole verdict over one log. Returns 1 if any half failed.
# $5 (optional) = the -profile dir, so the rival check below can look at the DISK and not just
# the log. Callers that can reach it should pass it; the verdict is weaker without it.
# $6 (optional) = rival pak size in bytes, ALREADY MEASURED by the caller. The deploy path uses
# this because $profile_dir there is a path on the staging host, and a local `[ -f ]` against a
# remote path silently answers "absent" — which would downgrade a real contest to WEAK EVIDENCE
# on every real deploy. "" = not measured, "0" = measured and absent.
verify_boot_log() {
  local log="$1" guid="$2" addons_dir="$3" admin_count="${4:-0}" profile_dir="${5:-}"
  local rival_bytes="${6:-}"
  local rc=0
  echo "==> boot verdict: $log"
  assert_local_addon_won "$log" "$guid" "$addons_dir" || rc=1
  assert_room_registered "$log" || rc=1
  assert_admins_configured "$log" "$admin_count" || rc=1

  # ── NON-VACUITY, measured rather than assumed ──────────────────────────────
  #
  # "The checkout won a contest" and "the checkout was the only candidate on the machine" print
  # the same PASS above and mean very different things. The second proves almost nothing, and an
  # assertion that passes because the alternative does not exist on disk is precisely the defect
  # this program keeps finding. So say which one happened.
  #
  # THE LOG ALONE IS NOT ENOUGH, and getting this wrong once is why this block reads the disk:
  # when -addonsDir wins, the engine never mounts the packed copy, so a log-only check reports
  # "no rival" on exactly the runs that pass. Measured on this boot — a 570,489 B version-1.0.2
  # pak sat in <profile>/addons/ throughout and the console log never mentions it. Reporting
  # that as "no rival to beat" would understate the strongest evidence the run produced, and
  # would train the reader to ignore the line.
  local pak="$profile_dir/addons/TBDFramework_$guid/data.pak"
  if grep -qE "Adding package '[^']*TBDFramework_$guid/'" "$log"; then
    echo "  NOTE  non-vacuous: a packed Workshop copy was MOUNTED this boot (per the log)."
  elif grep -qE "Downloading $guid version" "$log"; then
    echo "  NOTE  non-vacuous: the engine downloaded the Workshop copy this boot (per the log)."
  elif [ -n "$rival_bytes" ] && [ "$rival_bytes" -gt 0 ] 2>/dev/null; then
    echo "  NOTE  non-vacuous: a Workshop copy was on the server's disk and did NOT win —"
    echo "        $pak ($rival_bytes bytes)"
  elif [ -z "$rival_bytes" ] && [ -n "$profile_dir" ] && [ -f "$pak" ]; then
    echo "  NOTE  non-vacuous: a Workshop copy was on disk and did NOT win —"
    echo "        $pak ($(wc -c <"$pak" | tr -d ' ') bytes)"
  elif [ -n "$rival_bytes" ] || [ -n "$profile_dir" ]; then
    echo "  NOTE  WEAK EVIDENCE: no Workshop copy in the log and none at"
    echo "        $pak"
    echo "        so the addon-path assertion had nothing to beat. To make it a real contest,"
    echo "        boot once with -config and NO -addonsDir to populate that path, then re-run."
  else
    echo "  NOTE  rival unknown — no profile dir given, so this could not check whether a"
    echo "        Workshop copy even exists. Pass the -profile dir to strengthen the verdict."
  fi

  if [ "$rc" -ne 0 ]; then
    echo "BOOT VERDICT: FAILED"
  else
    echo "BOOT VERDICT: PASS"
  fi
  return "$rc"
}

# --verify-boot-selftest: prove the verdict can FAIL. A gate that has never been observed
# failing is not a gate. Every case here is a log the engine really can produce.
verify_boot_selftest() {
  local d pass=0 fail=0
  d="$(mktemp -d "${TMPDIR:-/tmp}/tbd-verify-boot.XXXXXX")"
  local guid="B2C3D4E5F6A78901"
  local staging="/home/sam/tbd/addons"

  # (a) THE DEFECT: -config only. Room registers, config valid, mod loads — from the profile
  #     pak. Byte-shape copied from a real 2026-08-01 boot on this machine.
  {
    echo "00:12:47.281 BACKEND      : Addon Download started $guid - TBD Framework"
    echo "00:12:47.281 BACKEND      : Downloading $guid version 1.0.2"
    echo "00:12:51.113 ENGINE       : FileSystem: Adding package '/home/sam/tbd/profile/addons/TBDFramework_$guid/' (pak count: 1) to filesystem under name TBD_Framework"
    echo "00:12:51.285  ENGINE       : Loaded addons:"
    echo "00:12:51.285   ENGINE       : gproj: './addons/core/core.gproj' guid: '5614BBCCBB55ED1C'"
    echo "00:12:51.285   ENGINE       : gproj: '/home/sam/tbd/profile/addons/TBDFramework_$guid/addon.gproj' guid: '$guid'"
    echo "00:12:28.401  BACKEND      : Server config loaded."
    echo "00:12:28.401   BACKEND      : JSON is Valid"
    echo "00:12:58.689 BACKEND      : Server registered with address: 192.168.0.140:2001"
    echo "00:12:58.689 SCRIPT       : [TBD][Stage] LOADING -> LOBBY"
  } >"$d/config-only.log"

  # (b) THE FIX: -addonsDir + -config. Same two healthy lines, different gproj path.
  {
    echo "00:20:30.385 ENGINE       : FileSystem: Adding relative directory '/home/sam/tbd/apps/mod/tbd-framework' to filesystem under name TBD_Framework"
    echo "00:20:30.564  ENGINE       : Loaded addons:"
    echo "00:20:30.564   ENGINE       : gproj: './addons/core/core.gproj' guid: '5614BBCCBB55ED1C'"
    echo "00:20:30.564   ENGINE       : gproj: '$staging/tbd-framework/addon.gproj' guid: '$guid'"
    echo "00:20:28.401  BACKEND      : Server config loaded."
    echo "00:20:28.401   BACKEND      : JSON is Valid"
    echo "00:20:58.689 BACKEND      : Server registered with address: 192.168.0.140:2001"
  } >"$d/both-flags.log"

  # (c) addons mode: right code, no room. The other broken half.
  {
    echo "21:52:30.564  ENGINE       : Loaded addons:"
    echo "21:52:30.564   ENGINE       : gproj: '$staging/tbd-framework/addon.gproj' guid: '$guid'"
    echo "21:52:36.933 SCRIPT       : [TBD][Validate] mission result=PASS errors=0 warnings=5"
    echo "21:52:40.000 SCRIPT       : [TBD][Stage] LOADING -> LOBBY"
  } >"$d/addons-only.log"

  # (d) mod absent entirely.
  {
    echo "00:30:30.564  ENGINE       : Loaded addons:"
    echo "00:30:30.564   ENGINE       : gproj: './addons/core/core.gproj' guid: '5614BBCCBB55ED1C'"
    echo "00:30:28.401  BACKEND      : Server config loaded."
    echo "00:30:28.401   BACKEND      : JSON is Valid"
    echo "00:30:58.689 BACKEND      : Server registered with address: 192.168.0.140:2001"
  } >"$d/no-mod.log"

  # name | file | fn | expected rc
  local -a cases=(
    "-config only: WORKSHOP copy won -> must FAIL|config-only.log|addon|1"
    "-addonsDir + -config: checkout won -> must PASS|both-flags.log|addon|0"
    "addons mode: checkout won -> addon check PASSES|addons-only.log|addon|0"
    "addons mode: no room -> must FAIL|addons-only.log|room|1"
    "-config only: room registered -> room check PASSES|config-only.log|room|0"
    "mod never loaded at all -> must FAIL|no-mod.log|addon|1"
    "missing log file -> must FAIL (check did not run)|ABSENT.log|addon|1"
    "missing log file -> room check must FAIL too|ABSENT.log|room|1"
    "addons mode has no config -> admin check must FAIL|addons-only.log|admin|1"
    "config accepted -> admin check PASSES|both-flags.log|admin|0"
  )

  local c name file fn want got
  for c in "${cases[@]}"; do
    IFS='|' read -r name file fn want <<< "$c"
    got=0
    case "$fn" in
      addon) assert_local_addon_won "$d/$file" "$guid" "$staging" >/dev/null 2>&1 || got=$? ;;
      room)  assert_room_registered "$d/$file"                     >/dev/null 2>&1 || got=$? ;;
      admin) assert_admins_configured "$d/$file" 1                 >/dev/null 2>&1 || got=$? ;;
    esac
    if [ "$got" -eq "$want" ]; then
      echo "  PASS  $name"
      pass=$((pass + 1))
    else
      echo "  FAIL  $name (wanted rc=$want, got rc=$got)"
      fail=$((fail + 1))
    fi
  done

  # The two directions must not agree. If the same log both passes and fails the addon check,
  # the check is reading nothing. This is the guard against a pattern that matches everything
  # (or nothing) still printing ten green lines above.
  if assert_local_addon_won "$d/both-flags.log" "$guid" "$staging" >/dev/null 2>&1 \
     && ! assert_local_addon_won "$d/config-only.log" "$guid" "$staging" >/dev/null 2>&1; then
    echo "  PASS  the addon check DISCRIMINATES (passes one log, fails the other)"
    pass=$((pass + 1))
  else
    echo "  FAIL  the addon check does not discriminate — it is vacuous."
    fail=$((fail + 1))
  fi

  # The format check that USED to be sufficient is not, and this proves it on the spot: the
  # -config-only log is the stale SOURCE, yet a current-format Workshop build makes it
  # indistinguishable by line count. Kept as an executable statement so nobody re-derives the
  # format check as a substitute for the path check.
  local tagged
  tagged="$(grep -c '\[TBD\]\[' "$d/config-only.log" || true)"
  if [ "$tagged" -gt 0 ]; then
    echo "  PASS  format check alone would MISS this (log has $tagged '[TBD][' lines yet loaded"
    echo "        the Workshop copy) — proves the path check is not redundant with it"
    pass=$((pass + 1))
  else
    echo "  FAIL  fixture (a) should carry current-format tagged lines"
    fail=$((fail + 1))
  fi

  # ── the non-vacuity reporter itself ────────────────────────────────────────
  # It got this wrong once (log-only, so it cried "no rival" on exactly the passing runs).
  # Pin all three ways it can learn about the rival, or the next edit reintroduces that.
  mkdir -p "$d/profile/addons/TBDFramework_$guid"
  head -c 4096 /dev/zero > "$d/profile/addons/TBDFramework_$guid/data.pak"
  local out
  # (i) rival on DISK, log silent about it — the shape a passing -addonsDir boot really has
  out="$(verify_boot_log "$d/both-flags.log" "$guid" "$staging" 1 "$d/profile" 2>&1)"
  case "$out" in
    *"non-vacuous: a Workshop copy was on disk and did NOT win"*)
      echo "  PASS  rival found on DISK when the log never mentions it"; pass=$((pass + 1)) ;;
    *) echo "  FAIL  rival on disk not reported: $out"; fail=$((fail + 1)) ;;
  esac
  # (ii) caller pre-measured it (the remote-deploy path, where a local stat cannot work)
  out="$(verify_boot_log "$d/both-flags.log" "$guid" "$staging" 1 "/nonexistent/remote" 570489 2>&1)"
  case "$out" in
    *"on the server's disk and did NOT win"*570489*)
      echo "  PASS  caller-measured rival size is trusted over a local stat"; pass=$((pass + 1)) ;;
    *) echo "  FAIL  pre-measured rival not reported: $out"; fail=$((fail + 1)) ;;
  esac
  # (iii) genuinely no rival -> must say the evidence is WEAK, not print a clean pass
  out="$(verify_boot_log "$d/both-flags.log" "$guid" "$staging" 1 "/nonexistent/remote" 0 2>&1)"
  case "$out" in
    *"WEAK EVIDENCE"*)
      echo "  PASS  absent rival is reported as WEAK EVIDENCE, not as a clean win"; pass=$((pass + 1)) ;;
    *) echo "  FAIL  absent rival not flagged weak: $out"; fail=$((fail + 1)) ;;
  esac

  rm -rf "$d"
  echo
  if [ "$fail" -ne 0 ]; then
    echo "BOOT VERDICT SELFTEST: $pass passed, $fail FAILED"
    return 1
  fi
  echo "BOOT VERDICT SELFTEST: $pass passed, 0 failed"
  return 0
}

if [ "$VERIFY_BOOT_SELFTEST" -eq 1 ]; then
  echo "==> boot verdict selftest (local only, no deploy, no ssh)"
  verify_boot_selftest
  exit $?
fi

if [ -n "$VERIFY_BOOT_LOG" ]; then
  # Deliberately does NOT source deploy.env: the point is to run against a log you already
  # have, on a machine with no staging credentials.
  _vb_guid="${TBD_ADDON_GUID:-$(read_addon_guid || echo B2C3D4E5F6A78901)}"
  if [ -z "${TBD_ADDONS_STAGING:-}" ]; then
    echo "--verify-boot needs TBD_ADDONS_STAGING (the -addonsDir the server was launched with)," >&2
    echo "so it knows which path counts as 'the checkout we deployed'. Export it, e.g." >&2
    echo "  TBD_ADDONS_STAGING=/home/sam/tbd/addons bash scripts/mod/deploy-staging.sh --verify-boot <log>" >&2
    exit 2
  fi
  verify_boot_log "$VERIFY_BOOT_LOG" "$_vb_guid" "$TBD_ADDONS_STAGING" "${TBD_ADMIN_COUNT:-0}" \
    "${TBD_PROFILE_DIR:-}"
  exit $?
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
# T-607: NOT `: "${TBD_SCENARIO:={69A85365FC09E2CA}Missions/...}"`. That idiom — which is what
# this line was — is silently truncated by bash: the `}` of the ResourceGUID closes the
# parameter expansion, so the default becomes `{69A85365FC09E2CA` and the rest of the line is
# parsed as literal text and discarded. Measured:
#   $ : "${TBD_SCENARIO:={69A85365FC09E2CA}Missions/TBD_Dev_POC.conf}"; echo "[$TBD_SCENARIO]"
#   [{69A85365FC09E2CA]
# So every deploy that did NOT override TBD_SCENARIO in deploy.env rendered a config the engine
# hard-rejects, and found out ~90 s into the boot, after a full rsync and script compile:
#   BACKEND (E): Value of "#/game/scenarioId" does not match the required pattern.
#                Value: "{69A85365FC09E2CA"
#   BACKEND (E): There are errors in server config!  ->  ENGINE (E): Unable to initialize the game
# A single-quoted assignment has no such parse. Do not "tidy" it back into the `:=` form.
if [ -z "${TBD_SCENARIO:-}" ]; then
  TBD_SCENARIO='{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'
fi
: "${TBD_BIND_IP:=192.168.0.140}"
: "${TBD_SERVER_DIR:=/home/sam/steam/arma-reforger-server}"

# Server launch mode:
#   config  — -addonsDir + -config. THE DEFAULT, and the only mode that is both correct and
#             joinable. Registers a backend room ("Server registered with address:" /
#             "Direct Join Code:"), supplies game.admins[] so `#tbd` works, AND loads the
#             checkout this deploy rsynced rather than the Workshop copy (T-604/T-607).
#   addons  — -addonsDir + -addons + -server. Loads the checkout, registers NO backend room,
#             and has no server config at all, so it has no admins either. Headless log
#             verification ONLY. Direct Join answers "No server found" in this mode; that is
#             the flag combination, not a fault.
#
# The default was `addons` and that was the wrong half to default to: the deploy's job is to
# stand up something a human can join, and addons mode never can. config mode used to be worse
# (it ran the Workshop copy), which is presumably why nobody flipped it. That is fixed above.
#
# NOTE the historical claim that config mode "requires the mod to be PUBLISHED to the Workshop"
# is FALSE and was the root of this ticket. It was measured on `-addons`. With `-addonsDir` the
# config-mode server loads local, unpublished content perfectly well; game.mods[] still names
# the id (a joining CLIENT resolves that from the Workshop) but the SERVER does not need it.
: "${TBD_SERVER_MODE:=config}"
: "${TBD_WORKSHOP_MOD_ID:=}"
: "${TBD_PUBLIC_ADDRESS:=${TBD_BIND_IP}}"
: "${TBD_GAME_PORT:=2001}"
: "${TBD_A2S_PORT:=17777}"          # MUST differ from TBD_GAME_PORT or replication fails
: "${TBD_SERVER_NAME:=TBD Staging POC}"
: "${TBD_ADMIN_PASSWORD:=tbd-admin}"
: "${TBD_MAX_PLAYERS:=64}"
: "${TBD_ADMIN_IDENTITY_IDS:=}"   # comma-separated identityIds → in-game admins (#tbd commands)
: "${TBD_SERVER_CONFIG_REMOTE:=$(dirname "$TBD_PROFILE_DIR")/server.config.json}"
# T-607: how long to wait for the engine to reach a verdict before failing the deploy. Room
# registration landed 14 s after start on a measured 2026-08-01 boot, but that number is not
# reliable — run-playtest-server.sh:698 records the same binary and config registering in 13 s
# on one boot and never across 300 s on another. This is a bound on patience, not an estimate.
: "${TBD_BOOT_VERIFY_TIMEOUT:=180}"

# T-607: the GUID is the join between the deployed checkout and game.mods[], and if deploy.env
# drifts from the gproj the addon assertion starts checking the wrong id — it would then pass
# only when the mod did NOT load. Cross-check rather than trust.
_gproj_guid="$(read_addon_guid || true)"
if [ -n "$_gproj_guid" ] && [ "$_gproj_guid" != "$TBD_ADDON_GUID" ]; then
  echo "TBD_ADDON_GUID='$TBD_ADDON_GUID' does not match apps/mod/tbd-framework/addon.gproj" >&2
  echo "  ('$_gproj_guid'). The gproj is the source of truth — fix deploy.env, or the boot" >&2
  echo "  assertion will be checking an addon id this checkout does not publish." >&2
  exit 1
fi

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
    # T-607: validate admin ids against the ENGINE's own schema, here, before anything is
    # rsynced. Both patterns copied verbatim out of the engine's rejection of a bad value
    # (1.7.0.54, via run-playtest-server.sh:539):
    #   BACKEND (E): RegEx Pattern: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
    #   BACKEND (E): RegEx Pattern: "^[0-9]{17}$"
    # A bad entry is a HARD FATAL at boot ("There are errors in server config!" -> "Unable to
    # initialize the game") reported ~90 s in, AFTER a full deploy and script compile. Failing
    # here costs a millisecond and names the value instead of burning a deploy cycle.
    if [ -n "$TBD_ADMIN_IDENTITY_IDS" ]; then
      IFS=',' read -ra _admin_check <<< "$TBD_ADMIN_IDENTITY_IDS"
      for _aid in "${_admin_check[@]}"; do
        _aid="$(echo "$_aid" | xargs)"
        [ -z "$_aid" ] && continue
        if ! printf '%s' "$_aid" | grep -qE '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' \
        && ! printf '%s' "$_aid" | grep -qE '^[0-9]{17}$'; then
          echo "TBD_ADMIN_IDENTITY_IDS contains '$_aid', which is neither an identityId nor a SteamID." >&2
          echo "  identityId: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx  (lowercase hex)" >&2
          echo "  SteamID:    17 digits" >&2
          echo "  The engine rejects anything else and refuses to start; this is its schema, not ours." >&2
          exit 1
        fi
      done
    else
      echo "NOTE: TBD_ADMIN_IDENTITY_IDS is empty, so game.admins[] will be []. Every '#tbd'"
      echo "      command answers 'TBD: admin only.' — TBD_AdminService.IsAdmin() resolves from"
      echo "      vanilla's SCR_PlayerListedAdminManagerComponent, which is populated ONLY from"
      echo "      game.admins[]. 'passwordAdmin' is a different mechanism and does not feed it."
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
import json, os, re, sys

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

# T-607: scenarioId against the ENGINE's OWN schema, copied verbatim out of its rejection
# (1.7.0.54):
#   BACKEND (E): RegEx Pattern: "^\{[0-9A-F]{16}\}[a-zA-Z0-9_./ -]+$"
#   BACKEND (E): Pattern Description: "Param must start with ResourceGUID enclosed in brackets."
# Presence was checked above and that was not enough: a TRUNCATED scenarioId ("{69A85365FC09E2CA",
# the bash-brace defect fixed at TBD_SCENARIO) is present, is a string, and is fatal. This
# validator printed "config VALID" over exactly that config — a tool reporting success over an
# input it never really examined, which is the defect this whole file is written against. The
# engine finds it ~90 s into a boot, after the rsync and a full script compile; this finds it on
# the dev machine before anything is pushed.
scenario = game.get("scenarioId")
if isinstance(scenario, str) and scenario:
    if not re.match(r"^\{[0-9A-F]{16}\}[a-zA-Z0-9_./ -]+$", scenario):
        errs.append(
            "game.scenarioId %r is rejected by the engine's schema "
            "(^\\{[0-9A-F]{16}\\}[a-zA-Z0-9_./ -]+$). A value that stops right after the "
            "GUID means TBD_SCENARIO was truncated by brace parsing in the shell." % scenario)

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

# Build ExecStart per mode.
#
# `-config` is mutually exclusive with **`-addons`** — NOT with `-addonsDir`. Those are two
# different flags and the distinction is the whole of T-604. `-addons <GUID>` asks the engine to
# activate a mod id and is refused alongside `-config` ("-config cannot be used together with
# addons!"); `-addonsDir <dir>` only tells it where to LOOK, and combines with `-config` fine.
#
# config mode therefore carries BOTH, which is what makes it simultaneously joinable and honest:
# `-config` registers the backend room and supplies game.admins[], `-addonsDir` makes the
# checkout this deploy just rsynced the copy that actually loads. Without `-addonsDir` the
# engine satisfies game.mods[] from the Workshop instead — same GUID, different code — and
# staging reports on a build it never deployed. assert_local_addon_won() proves which one won;
# it is not decoration, it is the acceptance criterion.
#
# Flag ORDER matches run-playtest-server.sh:693 deliberately. The engine does not care, but two
# launch lines that mean the same thing should read the same, or the next person diffs them and
# finds a difference that isn't one.
if [ "$TBD_SERVER_MODE" = "config" ]; then
  EXECSTART="${TBD_SERVER_DIR}/ArmaReforgerServer -addonsDir ${TBD_ADDONS_STAGING} -config ${TBD_SERVER_CONFIG_REMOTE} -profile ${TBD_PROFILE_DIR} -maxFPS 60 -logStats 30000 -nothrow"
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

  # ── T-607: assert the boot, do not assume it ───────────────────────────────
  #
  # `systemctl restart` exits 0 over a unit that is already dead — the same defect T-289's
  # agent selftest exists for. And even a genuinely-running server proves nothing about WHICH
  # mod it loaded. Until this block existed the deploy's last word was `sleep 8`, after which
  # it printed a success banner regardless of what the engine did.
  #
  # This waits for the engine to get far enough to have decided (addon resolution and room
  # registration both land inside ~20 s of start — measured: config load at +4 s, addons
  # resolved at +7 s, room registered at +14 s on a 2026-08-01 boot), then pulls the log back
  # and runs the same verdict --verify-boot runs locally. One implementation, two callers.
  echo "==> waiting for the engine to reach a verdict"
  _remote_log=""
  _waited=0
  while [ "$_waited" -lt "$TBD_BOOT_VERIFY_TIMEOUT" ]; do
    _remote_log="$(ssh_cmd "ls -1d '$TBD_PROFILE_DIR'/logs/logs_* 2>/dev/null | tail -1" || true)"
    if [ -n "$_remote_log" ] && \
       ssh_cmd "grep -qF 'Server registered with address:' '$_remote_log/console.log' 2>/dev/null"; then
      break
    fi
    sleep 10
    _waited=$((_waited + 10))
    echo "    ${_waited}s — no room registration yet (log: ${_remote_log:-none})"
  done

  if [ -z "$_remote_log" ]; then
    echo "FAIL: the server produced no log directory under $TBD_PROFILE_DIR/logs after ${_waited}s." >&2
    echo "      The unit may not have started at all. Check:" >&2
    echo "        ssh $TBD_SSH_HOST systemctl --user status tbd-reforger.service" >&2
    exit 1
  fi

  _local_log="$(mktemp -t tbd-staging-console.XXXXXX.log)"
  ssh_cmd "cat '$_remote_log/console.log'" > "$_local_log" 2>/dev/null || true
  if [ ! -s "$_local_log" ]; then
    echo "FAIL: could not read $_remote_log/console.log off $TBD_SSH_HOST." >&2
    echo "      Refusing to report the deploy OK over a log this script never examined." >&2
    rm -f "$_local_log"
    exit 1
  fi

  _admin_count=0
  if [ -n "$TBD_ADMIN_IDENTITY_IDS" ]; then
    _admin_count="$(printf '%s' "$TBD_ADMIN_IDENTITY_IDS" | tr ',' '\n' | grep -c '[^[:space:]]' || true)"
  fi

  echo "    pulled $_remote_log/console.log ($(wc -c <"$_local_log" | tr -d ' ') bytes)"

  # Measure the rival ON THE HOST. $TBD_PROFILE_DIR is a remote path, so the local `[ -f ]`
  # fallback inside verify_boot_log would answer "absent" for a pak that is really sitting
  # there, and downgrade a genuine contest to WEAK EVIDENCE on every deploy.
  _rival_bytes="$(ssh_cmd "wc -c < '$TBD_PROFILE_DIR/addons/TBDFramework_$TBD_ADDON_GUID/data.pak' 2>/dev/null || echo 0" | tr -d ' \r\n' || true)"
  [ -n "$_rival_bytes" ] || _rival_bytes=0

  if [ "$TBD_SERVER_MODE" = "config" ]; then
    if ! verify_boot_log "$_local_log" "$TBD_ADDON_GUID" "$TBD_ADDONS_STAGING" "$_admin_count" \
         "$TBD_PROFILE_DIR" "$_rival_bytes"; then
      echo "" >&2
      echo "DEPLOY FAILED ITS OWN ACCEPTANCE CHECK. The files are on the host and the unit may" >&2
      echo "be running, but it is NOT serving what you deployed, or it is not joinable." >&2
      echo "Full log kept at: $_local_log" >&2
      exit 1
    fi
  else
    # addons mode cannot register a room or hold admins by construction, so running the full
    # verdict here would manufacture two guaranteed failures. Assert the half that IS
    # meaningful and say plainly that the rest was not checked, rather than printing green.
    echo "==> boot verdict: $_local_log (mode=addons — addon check only)"
    if ! assert_local_addon_won "$_local_log" "$TBD_ADDON_GUID" "$TBD_ADDONS_STAGING"; then
      echo "Full log kept at: $_local_log" >&2
      exit 1
    fi
    echo "  SKIP  room + admin checks: addons mode registers no room and loads no server"
    echo "        config. This server is NOT joinable and has NO admins. Use config mode."
  fi
  rm -f "$_local_log"
fi

# ── T-289: install the host control agent ────────────────────────────────────
#
# OFF BY DEFAULT (TBD_INSTALL_AGENT=1 to opt in). The render above is proven by
# --agent-selftest; THIS step is not, because exercising it means mutating the live
# staging host, which T-289 was not permitted to touch. It also buys nothing until the
# API side lands — nothing would connect to the socket. Turn it on when the API slice
# is ready, with someone watching the first run.
#
# The agent is enabled via its SOCKET, never its service: socket activation means the
# agent process only exists for the lifetime of one connection, so there is no long-lived
# listener to leak, wedge, or restart.
echo "==> host control agent (T-289)"
if [ "$TBD_INSTALL_AGENT" != "1" ]; then
  echo "[SKIP] agent install — TBD_INSTALL_AGENT=1 to enable."
  echo "       Preview the exact bytes with: --render-agent <dir>"
  echo "       Prove the behaviour with:     --agent-selftest <dir>"
elif [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] render agent, scp -> $TBD_AGENT_REMOTE_PATH"
  echo "[dry-run]   unit under control: $TBD_AGENT_UNIT"
  echo "[dry-run]   socket: \$XDG_RUNTIME_DIR/$TBD_AGENT_SOCKET (SocketMode=0600)"
  echo "[dry-run] systemctl --user enable --now tbd-reforger-agent.socket"
else
  # Render LOCALLY and validate BEFORE anything is pushed — the T-288 posture: a broken
  # artefact fails here, on the dev machine, not after it has landed on the server.
  TBD_AGENT_LOCAL="$(mktemp -d -t tbd-agent.XXXXXX)"
  render_agent_files "$TBD_AGENT_LOCAL"
  validate_agent_files "$TBD_AGENT_LOCAL"
  ssh_cmd "mkdir -p \"\$HOME/.config/systemd/user\" && cat > '$TBD_AGENT_REMOTE_PATH' && chmod 0700 '$TBD_AGENT_REMOTE_PATH'" \
    < "$TBD_AGENT_LOCAL/tbd-reforger-agent.sh"
  ssh_cmd "cat > \"\$HOME/.config/systemd/user/tbd-reforger-agent.socket\"" \
    < "$TBD_AGENT_LOCAL/tbd-reforger-agent.socket"
  ssh_cmd "cat > \"\$HOME/.config/systemd/user/tbd-reforger-agent@.service\"" \
    < "$TBD_AGENT_LOCAL/tbd-reforger-agent@.service"
  rm -rf "$TBD_AGENT_LOCAL"
  ssh_cmd bash -s <<'AGENTINSTALL'
set -euo pipefail
systemctl --user daemon-reload
systemctl --user enable --now tbd-reforger-agent.socket
# Same rule the agent itself follows: do not trust the enable, go look. A socket that
# did not come up must fail the deploy rather than be reported as installed.
state="$(systemctl --user show -p ActiveState --value tbd-reforger-agent.socket 2>/dev/null || true)"
if [ "$state" != "active" ] && [ "$state" != "listening" ]; then
  echo "FAIL: tbd-reforger-agent.socket is '$state', not listening." >&2
  exit 1
fi
echo "  agent socket listening at \${XDG_RUNTIME_DIR}/tbd-reforger-agent.sock"
AGENTINSTALL
fi

echo "==> V6 remote log grep"
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] cargo run -q -p xtask -- mod remote-logs"
  exit 0
fi

# T-607: READ THE EXIT CODE, do not just inherit it. remote-log-grep.sh is a FOUR-outcome
# script and its header names deploy-staging.sh as the consumer that pinned `2`:
#
#   0 HEALTHY  ·  1 FAIL  ·  2 PARTIAL (booted, nobody joined yet)  ·  3 ENVIRONMENT
#
# This line used to be the last statement in the file, so under `set -e` the deploy simply
# exited with whatever it returned. `2` is the NORMAL state immediately after a deploy — nobody
# has had time to join — so every healthy deploy reported failure to any caller reading `!= 0`,
# and the fix people reach for when a green run keeps "failing" is to stop believing the gate.
# `3` is the opposite hazard and must never be soft: it means no log was examined at all, so it
# says nothing about the mod and cannot be allowed to read as success.
#
# Same contract now applies to scripts/mod/mcp-wb-logs.sh and scripts/mod/tbd-spawn-verify.sh
# (T-612) — both were inverted and passed ONLY on the stale June build. Do not build a staging
# check on a `!= 0` reading of any of the three.
set +e
cargo run -q -p xtask -- mod remote-logs
_v6=$?
set -e
case "$_v6" in
  0) echo "V6 HEALTHY — current build, mission loaded, reached LOBBY, a player was seated." ;;
  2) echo "V6 PARTIAL — boot is healthy, no player has joined yet. This is the expected result"
     echo "   for a fresh deploy and is NOT a failure." ;;
  1) echo "V6 FAIL — a required structural line is missing, or an error class is present." >&2
     exit 1 ;;
  3) echo "V6 ENVIRONMENT — the log could not be obtained, so nothing was examined. This says" >&2
     echo "   NOTHING about the mod, and is not a pass." >&2
     exit 1 ;;
  *) echo "V6 returned an unexpected status $_v6 — treating as failure rather than guessing." >&2
     exit 1 ;;
esac
echo "==> deploy complete"
