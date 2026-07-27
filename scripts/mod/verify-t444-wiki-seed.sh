#!/usr/bin/env bash
# T-444 / T-462 — Class-R: `make seed` must apply `seeds/wiki_pages.sql`, and the
# seed file must carry the V-suite `field-manual` slug (content_golden §5).
#
# Wave 24 adversarial: deleting the wiki seed line from the Makefile `seed:`
# recipe greens the cold gate — nothing pinned the recipe to the seed file.
#
# Gate: bash scripts/mod/verify-t444-wiki-seed.sh
# (Wired into scripts/platform/wave.sh gate / gate --slice as "T-444 wiki seed".)
#
# OWNS WIDEN: wave_plan T-444 lists Makefile + apps/website/api/seeds; this
# script is the Class-R perturbation guard for the seed-recipe contract.
# T-462 owns the script + wave.sh wiring.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MAKEFILE="$ROOT/Makefile"
SEED="$ROOT/apps/website/api/seeds/wiki_pages.sql"

if [[ ! -f "$MAKEFILE" ]]; then
	echo "FAIL: missing $MAKEFILE"
	echo "      restore Makefile so the seed recipe can be pinned."
	exit 1
fi

if [[ ! -f "$SEED" ]]; then
	echo "FAIL: missing $SEED"
	echo "      T-444 requires apps/website/api/seeds/wiki_pages.sql for make seed."
	exit 1
fi

if [[ ! -s "$SEED" ]]; then
	echo "FAIL: $SEED is empty"
	echo "      seed file must contain wiki page rows (incl. field-manual)."
	exit 1
fi

# Pin V-suite slug so an empty INSERT or unrelated SQL cannot satisfy presence.
if ! grep -q "field-manual" "$SEED"; then
	echo "FAIL: $SEED does not contain 'field-manual'"
	echo "      content_golden §5 / V-suite expects the field-manual wiki slug."
	exit 1
fi

# Extract the `seed:` recipe body (recipe lines are tab-indented). Comments and
# other targets must not satisfy the pin — only an executable recipe line that
# redirects/applies seeds/wiki_pages.sql counts.
seed_recipe="$(
	awk '
		/^seed:/ { in_seed=1; next }
		in_seed && /^[^#[:space:]\t]/ && $0 !~ /^#/ { exit }
		in_seed && /^\t/ { print }
	' "$MAKEFILE"
)"

if [[ -z "$seed_recipe" ]]; then
	echo "FAIL: Makefile has no tab-indented body under the seed: target"
	echo "      make seed must apply Discord/registry/faction/vehicle/wiki seeds."
	exit 1
fi

# Require the wiki seed on a live recipe line (not a # comment inside the recipe).
if ! printf '%s\n' "$seed_recipe" | grep -v $'^\t[[:space:]]*#' | grep -q 'seeds/wiki_pages\.sql'; then
	echo "FAIL: Makefile seed: recipe does not reference seeds/wiki_pages.sql"
	echo "      Add (under seed:):"
	echo "        cd \$(WEB) && \$(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/wiki_pages.sql"
	echo "      Without this line, make seed never loads doctrine wiki pages."
	exit 1
fi

echo "PASS: T-444 wiki seed — Makefile seed: applies seeds/wiki_pages.sql; field-manual present"
