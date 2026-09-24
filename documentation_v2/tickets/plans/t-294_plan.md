# T-294 — Plan

## Context
terrain-registry.json lists arland (queued, P1) but packages/map-assets/arland holds only a 756-byte manifest.
The export gate (xtask/src/gate_export_terrain.rs) has only ever run for everon.

## Approach
1. MANUAL (operator): Workbench, arland world with all layers, Plugins > TBD > "Export TBD World Objects (full)";
   then `cargo run -q -p tbd-tools --bin world -- copy-export-profile --terrain arland --full --profile "$PROFILE_DIR"`.
2. Agent: `cargo xtask map export-terrain arland --phase P1`; fix any everon-only path or count assumption in
   `gate_export_terrain.rs`; fill `packages/map-assets/arland/manifest.json` (objects.schemaVersion 1.1.0, transforms, counts).
3. Perturbation: run the gate with the staging dir renamed → refuses with the runbook; restore, `touch`, green.
4. List the emitted objects/ files in the report (owns cannot name them).
## Risks
- The arland world may exceed the 512 m cell pass budget; the plugin writes the meta sentinel last — treat a missing
  `_meta.json` as incomplete, not as zero objects.

## Verification
- `cargo xtask map export-terrain arland --phase P1` · `cargo xtask ci verify terrain-manifest`
- `cargo xtask platform wave gate --slice T-294`
