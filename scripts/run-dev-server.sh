#!/usr/bin/env bash
# Launch the TBD Dev POC on the native Linux Arma Reforger dedicated server.
# Local dev uses -server + -addons (config.json mods require Workshop download).
#
# Prereq: install "Arma Reforger Server" (Steam appid 1890870), e.g.:
#   steam steam://install/1890870
#   # or headless: steamcmd +login <user> +app_update 1890870 validate +quit
#
# Usage: bash scripts/run-dev-server.sh
set -euo pipefail

REPO="/home/Samuel/Projects/Arma reforger"
SERVER_DIR="$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server"
SERVER_BIN="$SERVER_DIR/ArmaReforgerServer"
PROFILE="$REPO/.local-test-profile"
ADDON_GUID="B2C3D4E5F6A78901"          # tbd-framework (addon.gproj)
SCENARIO="{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"   # the SCR_MissionHeader (not the raw world)
ADDONS_STAGING="$HOME/.local/share/tbd-server-addons"

if [ ! -x "$SERVER_BIN" ]; then
  echo "Arma Reforger Server not found at:" >&2
  echo "  $SERVER_BIN" >&2
  echo "Install it from Steam (appid 1890870):" >&2
  echo "  steam steam://install/1890870" >&2
  exit 1
fi

if [ ! -f "$PROFILE/profile/TBD_BackendConfig.json" ]; then
  echo "Profile not prepared — run: bash scripts/setup-server-profile.sh" >&2
  exit 1
fi

# Clean addons dir holding ONLY tbd-framework, so the server doesn't scan the
# whole projects folder (and never touches the read-only CRF Tbd_framework/).
mkdir -p "$ADDONS_STAGING"
ln -sfn "$REPO/tbd-framework" "$ADDONS_STAGING/tbd-framework"

echo "Server log + cache go under: $PROFILE"
echo "Watch for: [TBD] Mission loaded, built slot spawn, assigned slot, spawn requested"
echo "(Mission loader hits http://127.0.0.1:8080; falls back to \$profile:missions/msn_8f3a2c.json if the web API is down.)"
echo "(Hosting the SCENARIO .conf via -server + local mod via -addons. NOTE: -config conflicts with -addons, so it is NOT used for local mods.)"
echo

# Must run from SERVER_DIR so the engine resolves ./addons (vanilla 58D0FB3206B6F859).
cd "$SERVER_DIR"
exec "$SERVER_BIN" \
  -profile "$PROFILE" \
  -addonsDir "$ADDONS_STAGING" \
  -addons "$ADDON_GUID" \
  -server "$SCENARIO" \
  -maxFPS 60 \
  -logStats 30000 \
  -nothrow
