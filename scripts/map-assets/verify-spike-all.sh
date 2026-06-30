#!/usr/bin/env bash
# T-090.3.0 — one-shot spike gate runner. Runs the spike verifiers in order, exit 1 on first fail.
# Usage: bash scripts/map-assets/verify-spike-all.sh TERRAIN=everon
#
# Does NOT run the Workbench plugin or copy-world-export-profile.mjs (those need a live editor + $profile
# output). Re-runs the read-only gates against whatever staging artifacts + ops log already exist.
# `make schema-validate` (full JSON-Schema conformance incl. the spike inventory) is run separately.
set -euo pipefail

TERRAIN="everon"
for a in "$@"; do
  case "$a" in
    TERRAIN=*) TERRAIN="${a#TERRAIN=}" ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT"

echo "== verify-spike-all TERRAIN=$TERRAIN =="

echo "--- map-census (census guard: pending_export + spike subregion → exit 0) ---"
node scripts/map-assets/census-types.mjs "TERRAIN=$TERRAIN"

echo "--- verify-spike-k1 (K1) ---"
node scripts/map-assets/verify-spike-k1.mjs "TERRAIN=$TERRAIN"

echo "--- census-spike (K1b + K1/K1b drift) ---"
node scripts/map-assets/census-spike.mjs "TERRAIN=$TERRAIN"

echo "--- verify-spike-ops-log (K7 + K2/K3/K4 gate↔artifact) ---"
node scripts/map-assets/verify-spike-ops-log.mjs "TERRAIN=$TERRAIN"

echo "== verify-spike-all: ALL PASS (TERRAIN=$TERRAIN) =="
