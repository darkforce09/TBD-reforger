#!/usr/bin/env bash
# enfusion-mcp's pak VFS only scans .pak files sitting DIRECTLY in <gamePath>/addons/,
# but a normal Reforger install nests them under addons/data/ and addons/core/.
# Result: game_read / asset_search / game_browse see zero base-game content.
#
# This builds a symlink farm at ~/.cache/enfusion-mcp-root whose addons/ holds a
# flattened, uniquely-named symlink to every real .pak, plus an addons/data/ dir so
# the server recognises it as a valid base game. Point ENFUSION_GAME_PATH at it
# (already wired in .mcp.json).
set -euo pipefail
GAME="${1:-/home/Samuel/.local/share/Steam/steamapps/common/Arma Reforger}"
FAKE="${2:-$HOME/.cache/enfusion-mcp-root}"

if [ ! -d "$GAME/addons" ]; then
  echo "Game addons dir not found: $GAME/addons" >&2; exit 1
fi

rm -rf "$FAKE"
mkdir -p "$FAKE/addons/data"
count=0
while IFS= read -r -d '' p; do
  rel="${p#"$GAME/addons/"}"
  ln -s "$p" "$FAKE/addons/${rel//\//_}"
  count=$((count + 1))
done < <(find "$GAME/addons" -iname "*.pak" -print0 | sort -z)

echo "Linked $count pak files into $FAKE/addons/"
