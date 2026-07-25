#!/usr/bin/env bash
# T-181.3.3 — mirror vanilla Enfusion SOURCE (with method bodies) from the AR Explorer.
#
# WHY THIS AND NOT THE PAK
# ------------------------
# The pak file table lists all 6,495 scripts, but 4,012 of them are compressed with a codec
# that is neither zlib, raw deflate, nor LZ4 (see docs/mod/vanilla_carve_coverage.md for the
# full negative-result list). `enfusion-mcp`'s own `game_read` fails on them too.
#
# arexplorer.zeroy.com is a Doxygen build of the same game version (1.7.0.54) with
# SOURCE_BROWSER enabled: 6,495 `*_source.html` pages — exactly matching the pak script count —
# each containing the complete file INCLUDING method bodies. That is strictly more than the
# official BI API docs give (signatures only).
#
# BE A GOOD CITIZEN. This is one person's site, and a full mirror is gigabytes.
#   * Default = a curated set covering what T-181 actually needs.
#   * `--all` exists but think before using it.
#   * Every page is cached; nothing is ever refetched.
#   * Sequential, with a delay between requests.
#
#   bash scripts/mod/fetch-vanilla-source.sh                  # curated set
#   bash scripts/mod/fetch-vanilla-source.sh SCR_BaseGameMode.c SCR_SpawnPoint.c
#   bash scripts/mod/fetch-vanilla-source.sh --grep Respawn   # every file matching a pattern
#   bash scripts/mod/fetch-vanilla-source.sh --all            # all 6,495 (slow, heavy — be sure)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CACHE="$ROOT/apps/mod/vanilla_reference/source_html"
BASE="https://arexplorer.zeroy.com"
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36"
DELAY="${TBD_FETCH_DELAY:-0.4}"

mkdir -p "$CACHE"

# The file index carries every filename -> doxygen page mapping, so we never have to reproduce
# Doxygen's name mangling ourselves.
INDEX="$CACHE/files.html"
if [ ! -s "$INDEX" ]; then
  echo "==> file index"
  curl -sSL -A "$UA" -o "$INDEX" "$BASE/files.html"
  echo "  got files.html ($(stat -c%s "$INDEX") bytes)"
fi

# name<TAB>source_page
MAP="$CACHE/map.tsv"
if [ ! -s "$MAP" ]; then
  grep -o 'href="[a-z0-9_]*_8c\.html" target="_self">[^<]*\.c<' "$INDEX" \
    | sed -E 's|href="([a-z0-9_]*)_8c\.html" target="_self">([^<]*)<|\2\t\1_8c_source.html|' \
    | sort -u > "$MAP"
  echo "  mapped $(wc -l < "$MAP") source pages"
fi

# What T-181 actually needs: the event-loop spine. Everything else is fetched on demand.
CURATED='SCR_BaseGameMode.c
SCR_BaseGameModeComponent.c
SCR_RespawnSystemComponent.c
SCR_RespawnComponent.c
SCR_SpawnPoint.c
SCR_SpawnerRespawnComponent.c
SCR_PossessSpawnPointComponent.c
SCR_SpawnHandlerComponent.c
SCR_SpawnRequestComponent.c
SCR_PlayerController.c
SCR_PlayerControllerGroupComponent.c
ChimeraMenuBase.c
SCR_MenuHelper.c
SCR_FactionManager.c
SCR_Faction.c
SCR_GroupsManagerComponent.c
SCR_AIGroup.c
SCR_GameModeHealthSettings.c
SCR_CharacterDamageManagerComponent.c'

targets=()
case "${1:-}" in
  --all)   mapfile -t targets < <(cut -f1 "$MAP") ;;
  --grep)  [ -n "${2:-}" ] || { echo "usage: $0 --grep <pattern>" >&2; exit 2; }
           mapfile -t targets < <(cut -f1 "$MAP" | grep -i "$2" || true) ;;
  "")      mapfile -t targets <<< "$CURATED" ;;
  *)       targets=("$@") ;;
esac

echo "==> ${#targets[@]} source page(s)"
got=0; miss=0
for name in "${targets[@]}"; do
  [ -n "$name" ] || continue
  page=$(awk -F'\t' -v n="$name" '$1==n {print $2; exit}' "$MAP")
  if [ -z "$page" ]; then
    echo "  MISS $name (not in index)" >&2; miss=$((miss+1)); continue
  fi
  dest="$CACHE/$page"
  if [ -s "$dest" ]; then got=$((got+1)); continue; fi
  code=$(curl -sSL -A "$UA" -o "$dest" -w '%{http_code}' "$BASE/$page" || echo 000)
  if [ "$code" = "200" ]; then
    got=$((got+1)); echo "  got  $name"
  else
    rm -f "$dest"; miss=$((miss+1)); echo "  FAIL $name (http $code)" >&2
  fi
  sleep "$DELAY"
done

echo "cached $got page(s), $miss missing -> $CACHE"
echo "next:  cargo run -q -p tbd-tools --bin enf -- source"
