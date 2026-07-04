#!/usr/bin/env bash
# T-090.3.1 — `make map-export TERRAIN=<id> PHASE=Pn` orchestrator (t090_3_map_asset_export.md).
#
# Data-only Map Engine v2 export: NO raster passes, no tile pyramid (cancelled per
# t090_legacy_raster_pipeline.md). Steps:
#   1. phase gate against terrain-registry.json importPhaseMax
#   2. staged raw present? else print the Workbench operator/MCP instructions and exit 2
#   3. node build-world-objects.mjs  (classify -> prefabs/chunks/census + manifest patch + ops log)
#   4. node build-roads-from-topo.mjs (Q1 roads pulled forward; pak VFS, no Workbench)
#
# Exit codes: 0 built · 1 bad args/failed build · 2 staged raw missing (operator step required)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

TERRAIN="${1:-${TERRAIN:-}}"
shift || true
PHASE="P1_buildings"
while [ $# -gt 0 ]; do
  case "$1" in
    --phase) PHASE="$2"; shift 2 ;;
    *) echo "export-terrain: unknown arg $1" >&2; exit 1 ;;
  esac
done
if [ -z "$TERRAIN" ]; then
  echo "usage: export-terrain.sh <terrain> [--phase Pn]   (or TERRAIN env)" >&2
  exit 1
fi

# Phase gate: requested phase must not exceed the registry's importPhaseMax (phased-import rule:
# phase N+1 blocked until phase N shipped + human registry bump).
export REPO_ROOT
node - "$TERRAIN" "$PHASE" <<'EOF'
const { readFileSync } = require("node:fs");
const [terrain, phase] = process.argv.slice(2);
const ORDER = ["P1_buildings","P2_trees","P3_vegetation","P4_rocks","P5_props","P6_roads_highway","P7_roads_paved","P8_roads_dirt","P9_roads_path","P10_full"];
const reg = JSON.parse(readFileSync(`${process.env.REPO_ROOT}/packages/map-assets/terrain-registry.json`, "utf8"));
const row = reg.terrains.find((t) => t.terrainId === terrain);
if (!row) { console.error(`export-terrain: terrain '${terrain}' not in terrain-registry.json`); process.exit(1); }
if (!ORDER.includes(phase)) { console.error(`export-terrain: unknown phase '${phase}'`); process.exit(1); }
const max = row.importPhaseMax ?? "";
if (!ORDER.includes(max) || ORDER.indexOf(phase) > ORDER.indexOf(max)) {
  console.error(`export-terrain: phase ${phase} blocked — registry importPhaseMax=${max || "(none)"} (advance only after map-verify-phase PASS + registry bump)`);
  process.exit(1);
}
EOF

RAW="$REPO_ROOT/packages/map-assets/$TERRAIN/staging/export/raw-entities.jsonl"
if [ ! -f "$RAW" ]; then
  cat >&2 <<EOM
export-terrain: staged raw export missing for '$TERRAIN':
  $RAW

Operator step (one Workbench run per terrain per export):
  1. Workbench: open the terrain world with all layers loaded (wb_state should report ~1M+ entities)
  2. Run the full-world export — either:
       MCP:    MCP_CALL_TIMEOUT=3600 bash scripts/mod/mcp-call.sh wb_execute_action \\
                 '{"menuPath":"Plugins,TBD,Export TBD World Objects (full)"}'
       Manual: Workbench > Plugins > TBD > "Export TBD World Objects (full)"
     The plugin iterates 512 m cell passes and writes \$profile:TBD_WorldExport_full.jsonl,
     then TBD_WorldExport_full_meta.json (meta = completion sentinel — written last).
  3. Stage it:
       node scripts/map-assets/copy-world-export-profile.mjs TERRAIN=$TERRAIN --full \\
         --profile "\$PROFILE_DIR"
  4. Re-run: make map-export TERRAIN=$TERRAIN PHASE=$PHASE
EOM
  exit 2
fi

echo "export-terrain: $TERRAIN $PHASE — building catalog artifacts"
node "$SCRIPT_DIR/build-world-objects.mjs" --terrain "$TERRAIN" --phase "$PHASE" --patch-manifest --ops-log
node "$SCRIPT_DIR/build-roads-from-topo.mjs" --terrain "$TERRAIN" --ops-log
echo "export-terrain: $TERRAIN $PHASE done — next: make map-verify-phase TERRAIN=$TERRAIN PHASE=$PHASE"
