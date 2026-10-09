# Blueprint compiler test fixtures

The recorded inputs and blessed outputs that pin the building blueprint compiler and the world
line-of-sight model to the engine: [Workbench](/documentation/glossary/n_to_z.md#workbench) recordings
of one Everon farmhouse, a tilted garbage container and one terrain cell, the golden files the
compiler must reproduce, and a synthetic prefab tree.

## Contents

```text
tools/map_assets/blueprint_compiler/test_fixtures/blueprint/
├── FarmHouse_E_1L01_Wood.instances.golden.json  the farmhouse's socket and furniture instances
├── FarmHouse_E_1L01_Wood_blueprint.golden.json  the blueprint the voxel pipeline must reproduce
├── FarmHouse_E_1L01_Wood_children.json          the Workbench recon of the farmhouse's 88 children
├── FarmHouse_E_1L01_Wood_parity.json            400 engine verdicts, doors and glass excluded
├── FarmHouse_E_1L01_Wood_parity_doors.json      4000 engine verdicts with the doors closed
├── FarmHouse_E_1L01_Wood_voxels.jsonl.gz        the Workbench voxel dump of the farmhouse
├── prefab/                                      synthetic `.et` prefabs for the prefab tests
├── rotation_pin_GarbageContainer_01.json        a tilted parent and its rotated child
└── world_parity_18_0.json                       4000 engine verdicts in cell 18_0, a village
```

## How it works

Tests reach the files through `fixture(name)` in
`tools/map_assets/blueprint_compiler/src/test_fixtures.rs`, which joins the checkout root to this
folder. Every file is read, never written. Most
pins pair a fixture with the committed Everon assets under `assets/terrains/everon/`, so a
re-export of those assets re-blesses the matching golden here in the same change.

| Fixture | Test | What the test asserts |
|---|---|---|
| `_voxels.jsonl.gz` + `_blueprint.golden.json` | `farmhouse_dump_matches_golden_blueprint` | the full voxel pipeline (segments, default parameters) reproduces the golden exactly |
| `.instances.golden.json` + both parity files | `farmhouse_compound_door_parity_is_pinned` | byte-identical to the shipped `.instances.json`; 120 kept, 49 dropped, 7 closed doors; 3998 of 4000 and 400 of 400 agree |
| `rotation_pin_*.json` | `garbage_container_lid_pins_y_x_z_with_negated_pitch_and_roll` | `RIGID_HYPOTHESIS` wins, under 5 mm and 0.05°, by more than four times the runner-up's error |
| both farmhouse parity files | `farmhouse_descriptor_placed_at_a_yaw_replays_the_door_parity_fixture` | descriptor 132 placed at a yaw through the world occluder gives the compound's counts |
| `world_parity_18_0.json` | `world_parity_cell_18_0_is_pinned` | 3971 agree, 12 phantom, 17 missed of 4000, and at least 98 % |

The first three tests live in `tools/map_assets/blueprint_compiler/src/tests/` and
`tools/map_assets/blueprint_compiler/src/occlusion_sidecars/tests/`, and the other two in
`tools/map_assets/map_asset_verification/src/tests/world_line_of_sight_tests.rs`.
`_children.json` is read only by the path-resolution test below. Every number above is a blessed measurement: a change that
moves one re-blesses the fixture or the assertion on purpose.

## Format

- Encoding: JSON and gzip-compressed JSON lines, named after the prefab slug
  (`<slug>_<kind>.json`); `.golden` marks a blessed output.
- Schema:
  - `.instances.golden.json`: `contracts/definitions/building-instances.schema.json`;
  - `_blueprint.golden.json`: `contracts/definitions/building-blueprint.schema.json`;
  - `_children.json`: a recon dump: `prefabFilter`, `slug`, the root's pose and bounds,
    `rootComponents`, and `children[]` with each child's class, resource, relative and world pose,
    angles, bounds, pivot, mesh and components;
  - `_parity.json`, `_parity_doors.json`: `ParityFile` in
    `tools/map_assets/blueprint_compiler/src/parity_report.rs`, `{slug, pairs}` with each pair
    `[ox, oy, oz, tx, ty, tz, engineClear]` in the building's frame;
  - `_voxels.jsonl.gz`: a gzip-compressed `tbd-voxel-dump/1` file, one meta line then one line per
    scanline, as `tools/map_assets/blueprint_compiler/src/voxel_processing/voxel_types.rs` models it;
  - `rotation_pin_*.json`: `RotationFixture` in
    `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/rotation_validation.rs`: `source`, and `parent`
    and `child` poses with the child's observed placement;
  - `world_parity_*.json`: `WorldParityFile` in
    `tools/map_assets/map_asset_verification/src/world_line_of_sight.rs`, version
    `world-parity-1`: the cell, seed, strata and physics layer, and pairs
    `[ox, oy, oz, tx, ty, tz, clearEnts, clearWorld, hitSlug]` in world coordinates.
- Adding a file: record or emit it with the producer below, name it after its slug, and add the
  test that reads it; a new golden also needs the committed asset it must equal.

## Producers and consumers

- Producers:
  - the `tbd-export` Workbench handler `EMCP_WB_TbdBlueprint`
    (`mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/EMCP_WB_TbdBlueprint.c`),
    through `cargo xtask mcp wbcall`: action `recon` writes `<slug>_children.json`, `parity` writes
    `<slug>_parity.json`, `dump` writes `<slug>_voxels.jsonl` (compressed here), and
    `world-parity` writes `world_parity_<cx>_<cy>.json`, all into the Workbench profile's
    `TBD_Export` folder; the code does not name the recording of the door-inclusive
    `_parity_doors.json`;
  - `rotation_pin_GarbageContainer_01.json` is assembled from a recon and the prefab text, as its
    `source` field states;
  - the goldens are copies of compiler output: `cargo xtask map bvh-batch --prefab` writes the
    instances, `cargo xtask map blueprint-from-voxels` the blueprint.
- Consumers: the tests in the table;
  `nested_tooling_directories_resolve_repository_and_fixtures`
  (`tools/foundation/tool_test_support/src/tests/test_checkout_root_tests.rs`), which checks that
  a fixture here resolves from nested working directories. `cargo xtask map parity-report`, `bvh-parity`,
  `instances-verify`, `rotation-pin` and `world-los` accept files of these shapes by path.

## Boundaries

- Depends on: the committed Everon assets the pins pair with:
  `assets/terrains/everon/prefabs/buildings/` (the farmhouse sidecar and instances),
  `assets/terrains/everon/prefabs/descriptors/`, `assets/terrains/everon/objects/` (the
  prefab names and chunk rows) and the BLAS library in
  `assets/terrains/everon/prefabs/blas/`, whose `.bvh` files Git LFS stores, as it stores the
  DEM image that `assets/terrains/everon/manifest.json` names; nothing in this folder is in
  Git LFS.
- Used by: `tools/map_assets/blueprint_compiler/src/tests/`,
  `tools/map_assets/map_asset_verification/src/tests/`, and the path-resolution test above.
- Rules:
  - `FarmHouse_E_1L01_Wood.instances.golden.json` stays byte-identical to its shipped copy
    (`farmhouse_compound_door_parity_is_pinned`), so a re-emit re-blesses both;
  - the pipeline reproduces the blueprint golden (`farmhouse_dump_matches_golden_blueprint`).

## Related documentation

- [Building blueprint compiler](/tools/map_assets/blueprint_compiler/src/README.md) — the compiler
  these fixtures pin.
- [Map asset verification](/tools/map_assets/map_asset_verification/src/README.md) — the world
  line-of-sight replay.
- [Everon building models](/assets/terrains/everon/prefabs/buildings/README.md) — the shipped
  sidecars and instances the goldens equal.
- [Map asset commands](/tools/xtask/src/commands/map/README.md) — the commands that produce and
  replay these files.
