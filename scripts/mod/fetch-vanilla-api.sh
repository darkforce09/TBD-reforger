#!/usr/bin/env bash
# T-181.3.1 — mirror Bohemia's official Arma Reforger Script API reference locally.
#
# WHY
# ---
# `enf carve` (T-181.3) only recovers the *uncompressed* share of vanilla script. The classes
# that matter most for the spawn pipeline — SCR_BaseGameMode, SCR_PossessSpawnData,
# SCR_PossessSpawnRequestComponent, SCR_RespawnSystemComponent, ChimeraMenuBase — ship
# compressed and are unreachable by byte-scanning (docs/mod/vanilla_carve_coverage.md).
#
# But BI publishes the whole API as Doxygen HTML: **7,990 classes**, and every one of those
# "missing" classes IS there. Signatures + inheritance + doc strings, no bodies — which is
# exactly what `api_search` gives, except this is bulk, offline and greppable.
#
#   bash scripts/mod/fetch-vanilla-api.sh                 # class index only (1 request)
#   bash scripts/mod/fetch-vanilla-api.sh SCR_BaseGameMode SCR_PossessSpawnData
#   bash scripts/mod/fetch-vanilla-api.sh --from-file <newline-separated class names>
#
# Politeness: sequential, cached (never refetches), 0.3 s between requests. Do NOT bulk-fetch
# all 7,990 class pages — pull the ones a slice actually needs.
#
# Output is a GITIGNORED cache (public docs, but not ours to vendor). Only the derived TSV
# index under .ai/artifacts/enf-index/ is committed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CACHE="$ROOT/apps/mod/vanilla_reference/apidoc"
BASE="https://community.bistudio.com/wikidata/external-data/arma-reforger/ArmaReforgerScriptAPIPublic"
# The wiki returns 403 to curl's default UA — measured. A browser UA is required.
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36"

mkdir -p "$CACHE"

fetch() { # $1 = remote filename
  local f="$1" dest="$CACHE/$1"
  if [ -s "$dest" ]; then return 0; fi
  local code
  code=$(curl -sSL -A "$UA" -o "$dest" -w '%{http_code}' "$BASE/$f" || echo 000)
  if [ "$code" != "200" ]; then
    rm -f "$dest"
    echo "  MISS $f (http $code)" >&2
    return 1
  fi
  echo "  got  $f ($(stat -c%s "$dest") bytes)"
  sleep 0.3
}

echo "==> class index"
fetch annotated.html || { echo "fetch-vanilla-api: cannot reach the API docs" >&2; exit 1; }

# Doxygen mangles '_' to '__' in filenames: SCR_BaseGameMode -> interfaceSCR__BaseGameMode.html
doxy_name() { echo "interface${1//_/__}.html"; }

classes=()
if [ "${1:-}" = "--from-file" ]; then
  [ -n "${2:-}" ] || { echo "usage: $0 --from-file <path>" >&2; exit 2; }
  mapfile -t classes < <(grep -v '^\s*#' "$2" | sed '/^\s*$/d')
else
  classes=("$@")
fi

if [ "${#classes[@]}" -gt 0 ]; then
  echo "==> ${#classes[@]} class page(s)"
  for c in "${classes[@]}"; do
    fetch "$(doxy_name "$c")" || true
  done
fi

echo "cache: $CACHE ($(find "$CACHE" -name '*.html' | wc -l) pages)"
echo "next:  cargo run -q -p tbd-tools --bin enf -- apidoc"
