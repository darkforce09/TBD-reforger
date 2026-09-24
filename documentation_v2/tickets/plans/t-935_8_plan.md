# T-935.8 — Plan

## Context
occluder_host.rs fetches blas-manifest.json (:52), 1623 descriptor JSON files (:107, 19 MB) and
1690 `.bvh` sidecars (:134). Six golden blueprints exist under prefabs/buildings/. Full blueprint
extraction for every prefab is a Workbench pass the operator runs; this slice archives what is on
disk and prints that command.

## Approach
1. `xtask/src/map_blueprint/archive_emit.rs` (+ `mod.rs`, `library_cli.rs archive`): descriptors +
   BLAS index + blueprint levels → BuildingBlueprintArchive → `prefabs/building_blueprints.rkyv`.
2. `descriptor.rs`, `building_blueprint.rs`: `from_archived` equal to the JSON parse.
3. `occluder_host.rs`: archive branch when `manifest.buildings.archive` is Some; `.bvh` fetch
   indices come from the archive; JSON branch stays until T-935.13.
4. Parity test on all 1623 descriptors + 6 blueprints; perturbation skips the BLAS index → red.

## Risks
- Archive size (~ descriptors 19 MB JSON → expected < 8 MB rkyv): print the size; if larger than
  the JSON total, drop duplicated strings via an interned slug table.
- world-los must be unchanged: run it before and after.

## Verification
- `cargo test -p map-engine-core --all-features building`
- `cargo xtask map-blueprint archive --terrain everon`
- `cargo xtask map world-los --cell 18_0 --probe 9350,15,280 9380,15,290`
- `cargo xtask platform wave gate --slice T-935.8`
