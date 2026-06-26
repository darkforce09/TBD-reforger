#!/usr/bin/env bash
# Prepare dedicated-server profile files for TBD Framework POC.
#
# Arma Reforger dedicated server profile path (Linux typical):
#   ~/.local/share/ArmaReforger/profile
# or the path passed with -profile on the server binary.
#
# Usage:
#   bash scripts/setup-server-profile.sh [PROFILE_DIR]
#
# Example:
#   bash scripts/setup-server-profile.sh ~/.local/share/ArmaReforger/profile
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
PROFILE="${1:-${TBD_PROFILE:-$MOD_ROOT/.local-test-profile}}"
PROFILE_ROOT="$PROFILE/profile"

mkdir -p "$PROFILE_ROOT/missions"

# Backend config — edit serverToken to match GAME_SERVER_TOKENS in website/.env
cp "$MOD_ROOT/tbd-framework/Data/backend.example.json" "$PROFILE_ROOT/TBD_BackendConfig.json"

# Point at local API + dev token from .env if present
ENV_FILE="$WEB/.env"
if [ -n "${GAME_SERVER_TOKEN:-}" ]; then
  TOKEN="$GAME_SERVER_TOKEN"
elif [ -f "$ENV_FILE" ]; then
  TOKEN=$(grep '^GAME_SERVER_TOKENS=' "$ENV_FILE" | head -1 | cut -d= -f2 | cut -d, -f1)
fi
if [ -n "${TOKEN:-}" ]; then
  sed -i "s/replace-with-GAME_SERVER_TOKENS-value/$TOKEN/" "$PROFILE_ROOT/TBD_BackendConfig.json"
fi

# Seed mission fallback on disk (matches golden mission served by API)
cp "$SCHEMA/golden-missions/msn_8f3a2c.json" "$PROFILE_ROOT/missions/msn_8f3a2c.json"

# Optional registry override for dedicated (mod ships Data/registry.json; this is backup)
cp "$MOD_ROOT/tbd-framework/Data/registry.json" "$PROFILE_ROOT/TBD_Registry.json" 2>/dev/null || true

echo "Profile ready at: $PROFILE (game data under $PROFILE_ROOT)"
echo "  profile/TBD_BackendConfig.json"
echo "  profile/missions/msn_8f3a2c.json"
echo ""
echo "Workbench checklist:"
echo "  1. Open tbd-framework/addon.gproj"
echo "  2. Load mission Missions/TBD_Dev_POC.conf (or your scenario)"
echo "  3. Add TBD_FrameworkManager + TBD_RegistryPocComponent to GameMode entity"
echo "  4. Host dedicated server with -profile pointing at: $PROFILE"
