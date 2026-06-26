#!/usr/bin/env bash
# Workaround: Proton Workbench often cannot browse ~/.local/share/Steam paths.
# Symlink the base game .gproj (and its data folder) to a simple home path, then
# point Workbench at it when "Locate base game" appears.
#
# Usage: bash scripts/mod/setup-workbench-linux.sh
set -euo pipefail

STEAM_BASE="${STEAM_BASE:-$HOME/.local/share/Steam/steamapps/common/Arma Reforger}"
SRC="$STEAM_BASE/addons/data"
LINK_ROOT="$HOME/ArmaReforger-Base"

if [ ! -f "$SRC/ArmaReforger.gproj" ]; then
  echo "Base game not found at:" >&2
  echo "  $SRC/ArmaReforger.gproj" >&2
  echo "Install Arma Reforger via Steam first." >&2
  exit 1
fi

mkdir -p "$LINK_ROOT"
ln -sfn "$SRC" "$LINK_ROOT/data"

echo "Symlink ready:"
echo "  Linux:  $LINK_ROOT/data/ArmaReforger.gproj"
echo "  Proton: Z:\\home\\$(whoami)\\ArmaReforger-Base\\data\\ArmaReforger.gproj"
echo ""
echo "In Workbench 'Locate base game', browse to ArmaReforger.gproj at one of the paths above."
echo "Tip: Launch Arma Reforger (the game) once, quit, then open Workbench — auto-detect may work after that."
