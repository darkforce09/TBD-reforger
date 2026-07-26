#!/usr/bin/env bash
# T-251 — rsync + remote build/restart for the TBD website (API + Leptos SPA).
#
# Prereqs (dev PC):
#   cp scripts/deploy/deploy.env.example scripts/deploy/deploy.env
#   # fill TBD_SSH_HOST + TBD_SSH_PASS or TBD_SSH_IDENTITY_FILE
#   sshpass (if using password), rsync, ssh; remote needs Rust + trunk
#
# Usage:
#   bash scripts/deploy/deploy-website.sh --help
#   bash scripts/deploy/deploy-website.sh --dry-run
#   bash scripts/deploy/deploy-website.sh
#
# TBD_REMOTE_DIR must live under /home/sam/tbd/ (prefix-enforced). Also refuses any
# path/host containing "prairielearn" case-insensitively. Game server deploy remains
# scripts/mod/deploy-staging.sh.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MONO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ENV_FILE="${DEPLOY_ENV:-$MONO_ROOT/scripts/deploy/deploy.env}"
DRY_RUN=0

usage() {
  cat <<'EOF'
Usage: deploy-website.sh [--dry-run] [--help]

  Rsync the monorepo to TBD_REMOTE_DIR, bring up staging Postgres (compose),
  build the release API binary + Leptos SPA on the server, restart the
  user-systemd API unit, and print Caddy reload hints.

  --dry-run   Print the plan (rsync/ssh/compose/build/restart) without executing.
  -h, --help  Show this help.

Environment (scripts/deploy/deploy.env):
  TBD_SSH_HOST              required (e.g. sam@192.168.0.140)
  TBD_REMOTE_DIR            required (must be under /home/sam/tbd/ — never prairielearn)
  TBD_SSH_PASS              optional (sshpass)
  TBD_SSH_IDENTITY_FILE     optional (ssh -i)
  TBD_POSTGRES_HOST_PORT    optional (default 5432) — compose host port
  TBD_WEBSITE_SYSTEMD_UNIT  optional (default tbd-website-api.service)
  TBD_SKIP_COMPOSE          set to 1 to skip docker compose postgres up
  TBD_SKIP_SPA_BUILD        set to 1 to skip remote trunk build
  TBD_SKIP_API_BUILD        set to 1 to skip remote cargo build

Smoke (no SSH):
  bash scripts/deploy/deploy-website.sh --help
  bash scripts/deploy/deploy-website.sh --dry-run   # needs a filled deploy.env

Compose validate (local):
  docker compose -f apps/website/docker-compose.staging.yml config
  # on hosts with Podman only:
  podman compose -f apps/website/docker-compose.staging.yml config
EOF
}

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ ! -f "$ENV_FILE" ]; then
  echo "Missing $ENV_FILE — copy from scripts/deploy/deploy.env.example" >&2
  exit 1
fi
# shellcheck source=/dev/null
source "$ENV_FILE"

: "${TBD_SSH_HOST:?TBD_SSH_HOST required in deploy.env}"
: "${TBD_REMOTE_DIR:?TBD_REMOTE_DIR required in deploy.env}"
: "${TBD_POSTGRES_HOST_PORT:=5432}"
: "${TBD_WEBSITE_SYSTEMD_UNIT:=tbd-website-api.service}"
: "${TBD_SKIP_COMPOSE:=0}"
: "${TBD_SKIP_SPA_BUILD:=0}"
: "${TBD_SKIP_API_BUILD:=0}"

# Refuse any PrairieLearn path — remote dir, profile-ish vars we might grow, and the
# SSH host string if someone points it at a PL box path by mistake in the value.
# Case-insensitive (M3): PrairieLearn / PRAIRIELEARN / mixed case must all fail closed.
refuse_prairielearn() {
  local label="$1"
  local value="$2"
  local lowered="${value,,}"
  if [[ "$lowered" == *prairielearn* ]]; then
    echo "Refusing to deploy: $label must not contain 'prairielearn' (got: $value)" >&2
    echo "TBD lives under /home/sam/tbd/ only — see docs/website/HOME_SERVER.md." >&2
    exit 1
  fi
}

# Fail-closed: TBD_REMOTE_DIR must be under /home/sam/tbd/ after stripping trailing
# slashes. Reject ".." so /home/sam/tbd/../elsewhere cannot escape the prefix (M2).
require_tbd_remote_prefix() {
  local raw="$1"
  local dir="$raw"
  # Normalize trailing slashes (keep a lone "/" as "/").
  while [[ "$dir" == */ && "$dir" != / ]]; do
    dir="${dir%/}"
  done
  if [[ "$dir" == *..* ]]; then
    echo "Refusing to deploy: TBD_REMOTE_DIR must not contain '..' (got: $raw)" >&2
    echo "TBD_REMOTE_DIR must be under /home/sam/tbd/ — see docs/website/HOME_SERVER.md." >&2
    exit 1
  fi
  local allowed="/home/sam/tbd"
  if [[ "$dir" != "$allowed" && "$dir" != "$allowed"/* ]]; then
    echo "Refusing to deploy: TBD_REMOTE_DIR must be under /home/sam/tbd/ (got: $raw)" >&2
    echo "rsync --delete to paths outside /home/sam/tbd/ is forbidden." >&2
    exit 1
  fi
}

refuse_prairielearn "TBD_REMOTE_DIR" "$TBD_REMOTE_DIR"
refuse_prairielearn "TBD_SSH_HOST" "$TBD_SSH_HOST"
# Extra belt: common PL path even if someone aliases the var
if [ -n "${TBD_PROFILE_DIR:-}" ]; then
  refuse_prairielearn "TBD_PROFILE_DIR" "$TBD_PROFILE_DIR"
fi
require_tbd_remote_prefix "$TBD_REMOTE_DIR"

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

echo "==> deploy-website → ${TBD_SSH_HOST}:${TBD_REMOTE_DIR}"

echo "==> rsync (excludes secrets, build artifacts, LFS map-assets, oracle lanes)"
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] rsync -avz --delete … $TBD_SSH_HOST:$TBD_REMOTE_DIR/"
else
  rsync_to_remote -avz --delete \
    --exclude=.git/ \
    --exclude=target/ \
    --exclude=target-gate-*/ \
    --exclude=dist-gate-*/ \
    --exclude='**/node_modules/' \
    --exclude=apps/website/frontend/dist/ \
    --exclude=apps/website/api/.env \
    --exclude=apps/website/api/.tools/ \
    --exclude=scripts/deploy/deploy.env \
    --exclude=packages/map-assets/ \
    --exclude=apps/mod/crf_framework/ \
    --exclude=apps/mod/vanilla_reference/ \
    --exclude=apps/mod/playable_selector/ \
    --exclude=apps/mod/.local-test-profile/ \
    "$MONO_ROOT/" "$TBD_SSH_HOST:$TBD_REMOTE_DIR/"
fi

if [ "$TBD_SKIP_COMPOSE" != "1" ]; then
  echo "==> remote: staging Postgres (docker compose)"
  # Prefer `docker compose`; fall back to `podman compose` on hosts without docker.
  COMPOSE_REMOTE="cd '$TBD_REMOTE_DIR' && \
    export TBD_POSTGRES_HOST_PORT='${TBD_POSTGRES_HOST_PORT}' && \
    if command -v docker >/dev/null 2>&1; then \
      docker compose -f apps/website/docker-compose.staging.yml up -d postgres; \
    else \
      podman compose -f apps/website/docker-compose.staging.yml up -d postgres; \
    fi"
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "[dry-run] ssh … $COMPOSE_REMOTE"
  else
    ssh_cmd bash -lc "$COMPOSE_REMOTE"
  fi
fi

if [ "$TBD_SKIP_API_BUILD" != "1" ]; then
  echo "==> remote: cargo build --release -p website-api --bin api"
  API_BUILD="cd '$TBD_REMOTE_DIR' && \
    export PATH=\"\$HOME/.cargo/bin:\$PATH\" && \
    cargo build --release -p website-api --bin api && \
    test -x target/release/api"
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "[dry-run] ssh … $API_BUILD"
  else
    ssh_cmd bash -lc "$API_BUILD"
  fi
fi

if [ "$TBD_SKIP_SPA_BUILD" != "1" ]; then
  echo "==> remote: trunk build --release (Leptos SPA → frontend/dist)"
  SPA_BUILD="cd '$TBD_REMOTE_DIR/apps/website/frontend' && \
    export PATH=\"\$HOME/.cargo/bin:\$PATH\" && \
    trunk build --release"
  if [ "$DRY_RUN" -eq 1 ]; then
    echo "[dry-run] ssh … $SPA_BUILD"
  else
    ssh_cmd bash -lc "$SPA_BUILD"
  fi
fi

echo "==> remote: restart ${TBD_WEBSITE_SYSTEMD_UNIT}"
RESTART="systemctl --user restart '${TBD_WEBSITE_SYSTEMD_UNIT}' && \
  systemctl --user is-active '${TBD_WEBSITE_SYSTEMD_UNIT}'"
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] ssh … $RESTART"
else
  ssh_cmd bash -lc "$RESTART" || {
    echo "WARN: systemctl restart failed — is ${TBD_WEBSITE_SYSTEMD_UNIT} installed?" >&2
    echo "      See docs/website/HOME_SERVER.md Phase D for the unit sketch." >&2
  }
fi

echo "==> Caddy"
echo "    Ensure scripts/deploy/Caddyfile.website is loaded on the server"
echo "    (root → \$TBD_REMOTE_DIR/apps/website/frontend/dist; proxy /api → :8080)."
echo "    Example: caddy reload --config '$TBD_REMOTE_DIR/scripts/deploy/Caddyfile.website'"
echo "==> smoke hints"
echo "    curl -sf http://127.0.0.1:8080/healthz"
echo "    curl -sfI http://127.0.0.1:3080/"
echo "==> done"
