#!/usr/bin/env bash
# Launch the TBD Dev POC on the native Linux Arma Reforger dedicated server.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"

SERVER_DIR="$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server"
SERVER_BIN="$SERVER_DIR/ArmaReforgerServer"
PROFILE="$MOD_ROOT/.local-test-profile"
ADDON_GUID="B2C3D4E5F6A78901"
SCENARIO="{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"
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
