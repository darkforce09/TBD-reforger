#!/usr/bin/env bash
# One-time bootstrap on 192.168.0.140 (run from dev PC after deploy.env exists).
# Does NOT install steamcmd or Arma server — see docs/STAGING-SERVER.md for those.
#
# Usage: bash scripts/mod/bootstrap-staging-server.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
ENV_FILE="$DEPLOY_ENV"
# shellcheck source=/dev/null
[ -f "$ENV_FILE" ] && source "$ENV_FILE"

: "${TBD_SSH_HOST:?Set TBD_SSH_HOST in scripts/deploy/deploy.env}"
: "${TBD_REMOTE_DIR:=/home/sam/tbd/repo}"
: "${TBD_PROFILE_DIR:=/home/sam/tbd/profile}"
: "${TBD_ADDONS_STAGING:=/home/sam/tbd/addons-staging}"

if [[ "$TBD_REMOTE_DIR" == *prairielearn* ]]; then
  echo "Refusing: TBD_REMOTE_DIR must not be under prairielearn/" >&2
  exit 1
fi

SSH_BASE=(ssh -o StrictHostKeyChecking=no)
if [ -n "${TBD_SSH_PASS:-}" ]; then
  SSH_BASE=(sshpass -p "$TBD_SSH_PASS" ssh -o StrictHostKeyChecking=no)
elif [ -n "${TBD_SSH_IDENTITY_FILE:-}" ]; then
  SSH_BASE=(ssh -i "$TBD_SSH_IDENTITY_FILE" -o StrictHostKeyChecking=no)
fi

echo "==> Discovery on $TBD_SSH_HOST"
"${SSH_BASE[@]}" "$TBD_SSH_HOST" bash -s <<'DISC'
set -euo pipefail
echo "--- disk ---"
df -h ~
echo "--- ports 5432 8080 2001 ---"
ss -tlnp 2>/dev/null | grep -E ':5432|:8080|:2001' || echo "(none listening on those TCP ports)"
echo "--- docker ---"
docker compose version 2>/dev/null || docker --version 2>/dev/null || echo "docker not found"
DISC

echo "==> Create TBD directories (not prairielearn)"
"${SSH_BASE[@]}" "$TBD_SSH_HOST" bash -s <<EOF
set -euo pipefail
mkdir -p "$TBD_REMOTE_DIR" "$TBD_PROFILE_DIR" "$TBD_ADDONS_STAGING"
echo "OK: $TBD_REMOTE_DIR $TBD_PROFILE_DIR $TBD_ADDONS_STAGING"
EOF

echo ""
echo "Next steps (manual — see docs/STAGING-SERVER.md):"
echo "  1. steamcmd +app_update 1890870 on server"
echo "  2. Create apps/website/.env on server (SESSION_SECRET + GAME_SERVER_TOKENS)"
echo "  3. sudo loginctl enable-linger sam"
echo "  4. bash scripts/mod/deploy-staging.sh"
