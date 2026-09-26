# Blueprint compiler test fixtures

The recorded inputs and blessed outputs that pin the building blueprint compiler and the world
line-of-sight model to the engine: [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) recordings
of one Everon farmhouse, a tilted garbage container and two terrain cells, the golden files the
compiler must reproduce, and a synthetic prefab tree.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/
├── FarmHouse_E_1L01_Wood.bvh.golden             the farmhouse shell's occlusion sidecar
├── FarmHouse_E_1L01_Wood.instances.golden.json  the farmhouse's socket and furniture instances
├── FarmHouse_E_1L01_Wood_blueprint.golden.json  the blueprint the voxel pipeline must reproduce
├── FarmHouse_E_1L01_Wood_children.json          the Workbench recon of the farmhouse's 88 children
├── FarmHouse_E_1L01_Wood_parity.json            400 engine verdicts, doors and glass excluded
├── FarmHouse_E_1L01_Wood_parity_doors.json      4000 engine verdicts with the doors closed
├── FarmHouse_E_1L01_Wood_voxels.jsonl.gz        the Workbench voxel dump of the farmhouse
├── GarbageContainer_01_children.json            the Workbench recon of a tilted garbage container
├── prefab/                                      synthetic `.et` prefabs for the prefab tests
├── rotation_pin_GarbageContainer_01.json        a tilted parent and its rotated child
├── world_parity_18_0.json                       4000 engine verdicts in cell 18_0, a village
└── world_parity_forest.json                     4000 engine verdicts in cell 16_2, a forest
```

## How it works

Tests reach the files through `fixture(name)` in
`tools_v2/developer-tools/src/blueprint/tests/module/tests.rs`, which joins the compile-time
checkout root to this folder, or by the folder's full path. Every file is read, never written. Most
pins pair a fixture with the committed Everon assets under `assets_v2/terrains/everon/`, so a
re-export of those assets re-blesses the matching golden here in the same change.

| Fixture | Test | What the test asserts |
|---|---|---|
| `_voxels.jsonl.gz` + `_blueprint.golden.json` | `farmhouse_dump_matches_golden_blueprint` | the full voxel pipeline (segments, default parameters) reproduces the golden exactly |
| `_blueprint.golden.json` + `.bvh.golden` + `_parity.json` | `farmhouse_golden_parity_is_pinned` | `evaluate_los` agrees with all 400 pairs, none blocked where the engine is clear |
| `.bvh.golden` + `_parity.json` | `farmhouse_bvh_sidecar_parity_is_pinned` | byte-identical to the shipped `.bvh`; 3170 vertices, 2883 triangles, 1125 nodes; 400 of 400 agree |
| `.instances.golden.json` + both parity files | `farmhouse_compound_door_parity_is_pinned` | byte-identical to the shipped `.instances.json`; 120 kept, 49 dropped, 7 closed doors; 3998 of 4000 and 400 of 400 agree |
| `_children.json` | `farmhouse_sockets_match_the_workbench_recon` | all 88 children match the shipped instances with no extras or failures; 7 door, 88 pivot and at least 60 local checks |
| `_children.json` | `farmhouse_chunk_row_places_every_socket_child_within_2cm` | the farmhouse's row in chunk 18_0 is 5 wide at yaw 38.46 and places all 88 children within `POS_TOL_M` |
| `rotation_pin_*.json` | `garbage_container_lid_pins_y_x_z_with_negated_pitch_and_roll` | `RIGID_HYPOTHESIS` wins, under 5 mm and 0.05°, by more than four times the runner-up's error |
| `rotation_pin_*.json` | `garbage_container_row_carries_pitch_and_roll` | the container's row in chunk 19_0 is 8 wide with yaw 255.87, pitch -3.04, roll -4.75 and scale 1.0 |
| both farmhouse parity files | `farmhouse_descriptor_placed_at_a_yaw_replays_the_door_parity_fixture` | descriptor 132 placed at a yaw through the world occluder gives the compound's counts |
| `world_parity_18_0.json` | `world_parity_cell_18_0_is_pinned` | 3971 agree, 12 phantom, 17 missed of 4000, and at least 98 % |
| `world_parity_forest.json` | `world_parity_forest_cell_is_pinned`, `foliage_as_a_blocker_disagrees_with_the_projectile_trace` | 3977 agree, 11 phantom, 12 missed; with foliage blocking, under 90 % and over 500 phantoms |
| both world parity files | `world_parity_world_column_clears_its_floor_when_the_dem_is_present` | the terrain-inclusive column reaches 96 % and 94 %; skipped when the DEM does not decode |

The first eight tests live under `tools_v2/developer-tools/src/blueprint/tests/` (`module/`,
`bvh/`, `verify/`, `world_row/` and `rotation_pin/`), and the other five in
`tools_v2/developer-tools/src/map_verification/tests/world_line_of_sight.rs`.
`GarbageContainer_01_children.json` is the recon the rotation pin's `source` field cites, at the
same world position; no test reads it. Every number above is a blessed measurement: a change that
moves one re-blesses the fixture or the assertion on purpose.

## Format

- Encoding: JSON, gzip-compressed JSON lines and one binary sidecar, named after the prefab slug
  (`<slug>_<kind>.json`); `.golden` marks a blessed output.
- Schema:
  - `.bvh.golden`: the `TBVH` binary sidecar that `BvhSidecar` in
    `apps/website/map-engine/src/spatial/bvh/sidecar.rs` parses;
  - `.instances.golden.json`: `contracts_v2/definitions/building-instances.schema.json`;
  - `_blueprint.golden.json`: `contracts_v2/definitions/building-blueprint.schema.json`;
  - `_children.json`: a recon dump: `prefabFilter`, `slug`, the root's pose and bounds,
    `rootComponents`, and `children[]` with each child's class, resource, relative and world pose,
    angles, bounds, pivot, mesh and components;
  - `_parity.json`, `_parity_doors.json`: `ParityFile` in
    `tools_v2/developer-tools/src/blueprint/parity_report.rs`, `{slug, pairs}` with each pair
    `[ox, oy, oz, tx, ty, tz, engineClear]` in the building's frame;
  - `_voxels.jsonl.gz`: a gzip-compressed `tbd-voxel-dump/1` file, one meta line then one line per
    scanline, as `tools_v2/developer-tools/src/blueprint/voxel_processing/voxel_types.rs` models it;
  - `rotation_pin_*.json`: `PinFixture` in
    `tools_v2/developer-tools/src/blueprint/bvh/rotation_validation.rs`: `source`, and `parent`
    and `child` poses with the child's observed placement;
  - `world_parity_*.json`: `WorldParityFile` in
    `tools_v2/developer-tools/src/map_verification/world_line_of_sight.rs`, version
    `world-parity-1`: the cell, seed, strata and physics layer, and pairs
    `[ox, oy, oz, tx, ty, tz, clearEnts, clearWorld, hitSlug]` in world coordinates.
- Adding a file: record or emit it with the producer below, name it after its slug, and add the
  test that reads it; a new golden also needs the committed asset it must equal.

## Producers and consumers

- Producers:
  - the `tbd-export` Workbench handler `EMCP_WB_TbdBlueprint`
    (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/EMCP_WB_TbdBlueprint.c`),
    through `cargo xtask mcp wbcall`: action `recon` writes `<slug>_children.json`, `parity` writes
    `<slug>_parity.json`, `dump` writes `<slug>_voxels.jsonl` (compressed here), and
    `world-parity` writes `world_parity_<cx>_<cy>.json`, all into the Workbench profile's
    `TBD_Export` folder; the code does not name the recording of the door-inclusive
    `_parity_doors.json`;
  - `rotation_pin_GarbageContainer_01.json` is assembled from a recon and the prefab text, as its
    `source` field states;
  - the goldens are copies of compiler output: `cargo xtask map bvh-batch --prefab` writes the
    sidecar and the instances, `cargo xtask map blueprint-from-voxels` the blueprint.
- Consumers: the tests in the table;
  `compiler_fixtures_resolve_from_root_crate_and_source_directory`
  (`tools_v2/developer-tools/src/tests/repository_paths.rs`) and
  `nested_tooling_directories_resolve_repository_and_fixtures`
  (`tools_v2/xtask/src/tests/repository_root_tests.rs`), which check that fixtures here resolve
  from nested working directories. `cargo xtask map parity-report`, `bvh-parity`,
  `instances-verify`, `rotation-pin` and `world-los` accept files of these shapes by path.

## Boundaries

- Depends on: the committed Everon assets the pins pair with:
  `assets_v2/terrains/everon/prefabs/buildings/` (the farmhouse sidecar and instances),
  `assets_v2/terrains/everon/prefabs/descriptors/`, `assets_v2/terrains/everon/objects/` (the
  prefab names and chunk rows) and the BLAS library in
  `assets_v2/terrains/everon/prefabs/blas/`, whose `.bvh` files Git LFS stores, as it stores the
  DEM image that `assets_v2/terrains/everon/manifest.json` names; nothing in this folder is in
  Git LFS.
- Used by: `tools_v2/developer-tools/src/blueprint/tests/`,
  `tools_v2/developer-tools/src/map_verification/tests/`, and the path-resolution tests above.
- Rules:
  - `FarmHouse_E_1L01_Wood.bvh.golden` and `FarmHouse_E_1L01_Wood.instances.golden.json` stay
    byte-identical to their shipped copies (`farmhouse_bvh_sidecar_parity_is_pinned`,
    `farmhouse_compound_door_parity_is_pinned`), so a re-emit re-blesses both;
  - the pipeline reproduces the blueprint golden (`farmhouse_dump_matches_golden_blueprint`);
  - the prose rules of `tools_v2/xtask/src/tests/tooling_prose_rules.rs` exempt this tree from the
    ticket-id and Rust-file-name rules, since the recordings are data.

## Related documentation

- [Building blueprint compiler](/tools_v2/developer-tools/src/blueprint/README.md) — the compiler
  these fixtures pin.
- [Map asset verification](/tools_v2/developer-tools/src/map_verification/README.md) — the world
  line-of-sight replay.
- [Everon building models](/assets_v2/terrains/everon/prefabs/buildings/README.md) — the shipped
  sidecars and instances the goldens equal.
- [Map asset commands](/tools_v2/xtask/src/commands/map/README.md) — the commands that produce and
  replay these files.
