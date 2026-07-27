#!/usr/bin/env bash
# T-440 — Class-R: `make seed` must apply `seeds/faction_library.sql`, and the
# seed file must carry the starter BLUFOR row name `US Army 1980s` (T-256).
#
# Wave 10 / residual adversarial: cold/schema gates validate
# faction-library.sample.json but never pin that `make seed` applies
# apps/website/api/seeds/faction_library.sql. Deleting that Makefile seed
# line still greens the cold gate.
#
# Gate: bash scripts/mod/verify-t440-faction-library-seed.sh
# (Wired into scripts/platform/wave.sh gate / gate --slice as "T-440 faction library seed".)
#
# OWNS WIDEN: wave_plan T-440 lists Makefile + wave.sh + faction_library.sql;
# this script is the Class-R perturbation guard (same pattern as T-444/T-462).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MAKEFILE="$ROOT/Makefile"
SEED="$ROOT/apps/website/api/seeds/faction_library.sql"

# Distinctive starter-row pin from live SQL (BLUFOR INSERT name column).
STARTER_NAME='US Army 1980s'

if [[ ! -f "$MAKEFILE" ]]; then
	echo "FAIL: missing $MAKEFILE"
	echo "      restore Makefile so the seed recipe can be pinned."
	exit 1
fi

if [[ ! -f "$SEED" ]]; then
	echo "FAIL: missing $SEED"
	echo "      T-440 requires apps/website/api/seeds/faction_library.sql for make seed."
	exit 1
fi

if [[ ! -s "$SEED" ]]; then
	echo "FAIL: $SEED is empty"
	echo "      seed file must contain starter faction library rows (BLUFOR + OPFOR)."
	exit 1
fi

# Pin starter BLUFOR name so an empty INSERT or unrelated SQL cannot satisfy presence.
if ! grep -q "$STARTER_NAME" "$SEED"; then
	echo "FAIL: $SEED does not contain '$STARTER_NAME'"
	echo "      T-256 starter library expects the BLUFOR row name US Army 1980s."
	exit 1
fi

# Extract the `seed:` recipe body (recipe lines are tab-indented). Comments and
# other targets must not satisfy the pin — only an executable recipe line that
# redirects/applies seeds/faction_library.sql counts.
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

# Require the faction library seed on a live recipe line (not a # comment inside the recipe).
if ! printf '%s\n' "$seed_recipe" | grep -v $'^\t[[:space:]]*#' | grep -q 'seeds/faction_library\.sql'; then
	echo "FAIL: Makefile seed: recipe does not reference seeds/faction_library.sql"
	echo "      Add (under seed:):"
	echo "        cd \$(WEB) && \$(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/faction_library.sql"
	echo "      Without this line, make seed never loads the T-256 starter faction library."
	exit 1
fi

echo "PASS: T-440 faction library seed — Makefile seed: applies seeds/faction_library.sql; '$STARTER_NAME' present"
