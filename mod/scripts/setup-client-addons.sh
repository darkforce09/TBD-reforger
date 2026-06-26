#!/usr/bin/env bash
# Prepare local addon staging dir for Arma Reforger CLIENT (Steam launch options).
#
# Usage: bash scripts/setup-client-addons.sh
#
# Then set Steam → Arma Reforger → Properties → Launch Options:
#   -addonsDir "/home/Samuel/.local/share/tbd-server-addons" -addons B2C3D4E5F6A78901
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
STAGING="$HOME/.local/share/tbd-server-addons"

mkdir -p "$STAGING"
ln -sfn "$MOD_ROOT/tbd-framework" "$STAGING/tbd-framework"

echo "Client addon staging: $STAGING/tbd-framework"
echo ""
echo "Steam → Arma Reforger → Properties → Launch Options:"
echo "  -addonsDir \"$STAGING\" -addons B2C3D4E5F6A78901"
echo ""
echo "Restart the game, then Direct Join → 192.168.0.140 port 2001"
