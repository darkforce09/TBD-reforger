#!/usr/bin/env bash
# Prepare dedicated-server profile files for TBD Framework POC.
#
# Arma Reforger dedicated server profile path (Linux typical):
#   ~/.local/share/ArmaReforger/profile
# or the path passed with -profile on the server binary.
#
# Usage:
#   bash scripts/mod/setup-server-profile.sh [PROFILE_DIR]
#
# Example:
#   bash scripts/mod/setup-server-profile.sh ~/.local/share/ArmaReforger/profile
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
PROFILE="${1:-${TBD_PROFILE:-$MOD_ROOT/.local-test-profile}}"
PROFILE_ROOT="$PROFILE/profile"

mkdir -p "$PROFILE_ROOT/missions"

# Backend config — serverToken must match SERVICE_TOKEN in apps/website/api/.env.
# NOT GAME_SERVER_TOKENS: apps/website/api/src/config.rs:89 reads `SERVICE_TOKEN`, and
# middleware/auth.rs:113 compares it WHOLE via constant_time_equal — so it is one token, not a
# comma-separated list. The old plural name matched nothing, so every profile this script wrote
# shipped the literal placeholder and the server 401'd on its first call.
cp "$MOD_ROOT/tbd-framework/Data/backend.example.json" "$PROFILE_ROOT/TBD_BackendConfig.json"

# Point at local API + dev token from .env if present
ENV_FILE="$WEB/.env"
if [ -n "${SERVICE_TOKEN:-}" ]; then
  TOKEN="$SERVICE_TOKEN"
elif [ -f "$ENV_FILE" ]; then
  # `cut -d= -f2-`, not `-f2`: a base64 token ends in `=` and `-f2` would truncate it silently.
  # `|| true`: an .env with no SERVICE_TOKEN line makes grep exit 1, and `set -o pipefail` would
  # turn that into a hard abort instead of leaving the placeholder for the operator to fill in.
  TOKEN=$(grep '^SERVICE_TOKEN=' "$ENV_FILE" | head -1 | cut -d= -f2- || true)
fi
if [ -n "${TOKEN:-}" ]; then
  # The placeholder is spelled in Data/backend.example.json — keep the two in step, or this
  # substitution no-ops and the profile keeps the placeholder.
  # `|` delimiter, not `/`: base64 tokens contain `/`, which would break the s/// expression.
  sed -i "s|replace-with-SERVICE_TOKEN-value|$TOKEN|" "$PROFILE_ROOT/TBD_BackendConfig.json"
fi

# Seed mission fallback on disk (matches golden mission served by API).
#
# DESTINATION is named for the mission id, SOURCE is the golden that carries it — they are not
# the same string and that is what the old `golden-missions/msn_8f3a2c.json` got wrong.
# TBD_MissionLoader.LoadFromProfileFile reads `$profile:missions/<missionId>.json`, so the file
# on disk must be `<id>.json`; the golden holding `meta.id == msn_8f3a2c` is bridgehead-at-levie
# (18 slots — the reference mission in apps/mod/README.md). Adding a second msn_8f3a2c.json to
# golden-missions/ was the wrong fix: xtask/src/schema_gates.rs validates every file in that
# directory, so it would be a duplicate id validated twice and free to drift from what the API
# actually serves — the exact stale-vs-fallback ambiguity TBD_MissionLoader works to keep apart.
MISSION_ID="msn_8f3a2c"
GOLDEN="$SCHEMA/golden-missions/bridgehead-at-levie.json"
if [ ! -f "$GOLDEN" ]; then
  echo "ERROR: golden mission not found: $GOLDEN" >&2
  echo "       This script seeds the $MISSION_ID disk fallback from that file. If the golden" >&2
  echo "       was renamed, point GOLDEN at the one whose meta.id is $MISSION_ID." >&2
  exit 1
fi
cp "$GOLDEN" "$PROFILE_ROOT/missions/$MISSION_ID.json"

# Optional registry override for dedicated (mod ships Data/registry.json; this is backup)
cp "$MOD_ROOT/tbd-framework/Data/registry.json" "$PROFILE_ROOT/TBD_Registry.json" 2>/dev/null || true

echo "Profile ready at: $PROFILE (game data under $PROFILE_ROOT)"
echo "  profile/TBD_BackendConfig.json"
echo "  profile/missions/$MISSION_ID.json"
echo ""
echo "Workbench checklist:"
echo "  1. Open tbd-framework/addon.gproj"
echo "  2. Load mission Missions/TBD_Dev_POC.conf (or your scenario)"
echo "  3. Add TBD_FrameworkManager + TBD_RegistryPocComponent to GameMode entity"
echo "  4. Host dedicated server with -profile pointing at: $PROFILE"
