#!/usr/bin/env bash
# Rsync TBD platform to 192.168.0.140, rebuild API, refresh profile, restart game server.
#
# Prereqs (dev PC):
#   cp scripts/deploy/deploy.env.example scripts/deploy/deploy.env   # fill SSH + token
#   cd packages/tbd-schema && npm ci
#
# Usage:
#   bash scripts/mod/deploy-staging.sh                       # mode from deploy.env (default: addons)
#   bash scripts/mod/deploy-staging.sh --dry-run
#
#   # joinable server (after publishing tbd-framework to the Workshop):
#   TBD_SERVER_MODE=config TBD_WORKSHOP_MOD_ID=<workshopModId> bash scripts/mod/deploy-staging.sh
#
# Never rsyncs to /home/sam/prairielearn/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
ENV_FILE="$DEPLOY_ENV"
DRY_RUN=0

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    -h|--help)
      echo "Usage: deploy-staging.sh [--dry-run]"
      exit 0
      ;;
    *) echo "Unknown option: $arg" >&2; exit 2 ;;
  esac
done

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

if [[ "$TBD_REMOTE_DIR" == *prairielearn* ]]; then
  echo "Refusing to deploy: TBD_REMOTE_DIR must not be under prairielearn/" >&2
  exit 1
fi

case "$TBD_SERVER_MODE" in
  addons) ;;
  config)
    if [ -z "$TBD_WORKSHOP_MOD_ID" ]; then
      echo "TBD_SERVER_MODE=config requires TBD_WORKSHOP_MOD_ID (publish tbd-framework" >&2
      echo "to the Workshop first, then set its modId in deploy.env)." >&2
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
  (cd "$SCHEMA" && [ -d node_modules ] || npm ci --silent)
  node "$SCHEMA/scripts/validate-file.mjs" \
    "$SCHEMA/golden-missions/${TBD_MISSION_ID}.json"
fi

echo "==> rsync to $TBD_REMOTE_DIR"
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] rsync -avz --delete ... $TBD_SSH_HOST:$TBD_REMOTE_DIR/"
else
  rsync_to_remote -avz --delete \
    --exclude=.git/ \
    --exclude=apps/mod/crf_framework/ \
    --exclude=apps/mod/Tbd_framework/ \
    --exclude=apps/mod/.local-test-profile/ \
    --exclude='**/node_modules/' \
    --exclude=apps/website/.tools/ \
    --exclude=apps/website/.env \
    --exclude=apps/website/frontend/dist/ \
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
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] docker compose -f docker-compose.staging.yml up -d --build"
else
  ssh_cmd "cd '$TBD_REMOTE_DIR/apps/website' && docker compose -f docker-compose.staging.yml up -d --build"
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
  [ "$TBD_SERVER_MODE" = "config" ] && echo "[dry-run] render server config -> $TBD_SERVER_CONFIG_REMOTE (modId=$TBD_WORKSHOP_MOD_ID)"
  echo "[dry-run] ExecStart=$EXECSTART"
  echo "[dry-run] install tbd-reforger.service and restart"
else
  # In config mode, render the server config JSON on the host (registers the
  # backend room; the Workshop mod is downloaded from game.mods[]).
  if [ "$TBD_SERVER_MODE" = "config" ]; then
    # Build a JSON array of admin identityIds from the comma-separated env var.
    TBD_ADMINS_JSON=""
    if [ -n "$TBD_ADMIN_IDENTITY_IDS" ]; then
      IFS=',' read -ra _admin_ids <<< "$TBD_ADMIN_IDENTITY_IDS"
      for _aid in "${_admin_ids[@]}"; do
        _aid="$(echo "$_aid" | xargs)"
        [ -n "$_aid" ] && TBD_ADMINS_JSON="${TBD_ADMINS_JSON:+$TBD_ADMINS_JSON, }\"$_aid\""
      done
    fi
    ssh_cmd "cat > '$TBD_SERVER_CONFIG_REMOTE'" <<EOF
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
    "admins": [${TBD_ADMINS_JSON}],
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
    "mods": [
      { "modId": "${TBD_WORKSHOP_MOD_ID}", "name": "TBD_Framework" }
    ]
  },
  "operating": { "lobbyPlayerSynchronise": true }
}
EOF
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
