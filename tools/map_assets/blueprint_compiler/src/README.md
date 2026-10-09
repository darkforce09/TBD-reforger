# Building blueprint compiler

The offline compiler behind line of sight through buildings and prefabs in the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): it reads voxel dumps and
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) game models, extracts each building's floors,
walls and roof into a blueprint, builds bounding volume hierarchy (BVH) occlusion sidecars and the
prefab occluder library, folds them into one archive, and checks every step against
[Workbench](/documentation/glossary/n_to_z.md#workbench) recordings of the engine.

## Contents

```text
tools/map_assets/blueprint_compiler/src/
├── architectural_analysis/   slabs, walls, floor plates, outlines, roof grid, surface kinds, face pairing
├── architectural_analysis.rs  the module of the geometry stages
├── archive_emission/         blueprint assembly, the prefab occluder library and the rkyv archive
├── archive_emission.rs       the module of the writers
├── blueprint_from_voxels.rs  `run` (`blueprint-from-voxels`): the dump interpretation and per-band assembly
├── blueprint_ingestion.rs    `ingest-blueprints`: Workbench-exported blueprints validated and copied in
├── error.rs                  `Error`, `Result`, the context and refusal helpers
├── lib.rs                    the crate root: the modules and the command entries
├── mesh_decoding/            `.xob` model, collider and node table readers; `xob-inspect` and `pak-cat`
├── mesh_decoding.rs          the module of the model readers
├── occlusion_sidecars/       occlusion sidecars, the prefab walk, parity and placement checks
├── occlusion_sidecars.rs     the module of the sidecar and placement commands
├── parity_report.rs          `parity-report`: the Workbench oracle replayed through a blueprint
├── prelude.rs                the error type and the command entries in one import
├── test_fixtures.rs          `fixture(name)`: the committed test inputs (tests and the `test_fixtures` feature)
├── tests/                    the `blueprint-from-voxels` tests: synthetic bands and the farmhouse golden
├── voxel_processing/         the voxel dump model, its reader, the tunables, the march lattice and `voxels-from-mesh`
└── voxel_processing.rs       the module of the voxel dump side
```

## How it works

`lib.rs` declares one module per responsibility folder (`architectural_analysis`,
`archive_emission`, `mesh_decoding`, `occlusion_sidecars`, `voxel_processing`), each a file beside its folder
that names the folder's modules, and re-exports the command entries. Each folder keeps its unit
tests in its own `tests/` folder, one file per module. Every entry takes the checkout root and
the raw arguments from its `cargo xtask map` adapter, parses them itself and returns the exit
code; a failure that stops a command is an `Error`. The compiler has two lanes that meet in the
archive:

```text
Workbench dump action, or voxels-from-mesh
      │  <slug>_voxels.jsonl[.gz]
      ▼
blueprint-from-voxels ──▶ assets/terrains/everon/prefabs/buildings/<slug>.json
                                                    │
game paks ──bvh-batch --all-prefabs──▶ prefabs/blas/, prefabs/descriptors/, blas-manifest.json
      │                                             │
      └──bvh-batch --prefab──▶ buildings/<slug>.bvh, .instances.json, .scene.json
                                                    ▼
                      blueprint-from-voxels archive ──▶ prefabs/building_blueprints.rkyv
```

`run` (`blueprint-from-voxels`) finds `prefabs/dumps/<slug>_voxels.jsonl[.gz]` under the
Workbench profile's `TBD_Export` folder (or `--src`), filters by `--filter`, and interprets each
dump: `vertical_slabs::analyze` finds the floors, eave and ridge; `build_bands` extracts walls and
a floor plate for every floor band and adds an attic band of walls when the ridge rises at least
`attic_min_rise_m` above the last band; `blueprint_assembly::assemble` builds the blueprint, and
`validate_and_write` checks it against `contracts/definitions/building-blueprint.schema.json`
and writes it to `--out` or `assets/terrains/everon/prefabs/buildings/`. `--algo` picks the wall
extractor, `--params` overrides tunables, and `--debug-dir` writes a `<slug>_stages.json` with the
slabs and every wall cluster's verdict. A first argument `archive` runs the archive fold instead.

`blueprint_ingestion.rs` copies the blueprints the `tbd-export` building plugins wrote into the Workbench
profile (`prefabs/buildings/*.json`, searched two levels under `TBD_Export`) into
`assets/terrains/everon/prefabs/buildings/`, each only after it parses as `BuildingBlueprint`.
`parity_report.rs` replays a Workbench parity file (engine verdicts for observer and target pairs
in the building's frame, glass excluded) through `BuildingBlueprint::annotate_sight_line` over a sidecar,
and prints where the model and the engine disagree; it reports and exits 0.

## Public surface

Each entry backs one `cargo xtask map` command (`tools/xtask/src/commands/map/mod.rs`):

| Entry | Command | Exit 0 | Exit 1 |
|---|---|---|---|
| `run` | `blueprint-from-voxels [archive]` | every matched dump written; archive built | no match, a failure |
| `blueprint_ingestion::run` | `ingest-blueprints` | every matched file copied | nothing matched, a file failed |
| `parity_report::run` | `parity-report` | report printed | an argument missing |
| `run_mesh_voxelization` | `voxels-from-mesh` | dump written, or `--stats` printed | none: errors |
| `run_occlusion_sidecar_parity`, `run_occlusion_sidecar_emission` | `bvh-parity`, `bvh-emit` | report printed; sidecar written | an unknown argument |
| `run_occlusion_sidecar_batch` | `bvh-batch` | files written, or a dry run | an unknown argument |
| `run_xob_inspection`, `run_pak_file_print` | `xob-inspect`, `pak-cat` | printed or saved | an unknown argument |
| `run_instance_verification` | `instances-verify` | all within 2 cm and 1° | a mismatch |
| `run_rotation_validation` | `rotation-pin` | `Rigid::from_enfusion` first | another hypothesis first |

Every entry but `run_mesh_voxelization` also exits 1 on an unknown argument; that one returns an
error instead, and an error any entry returns reaches xtask as a failure.
`parity_report::ParityFile` and, behind the `test_fixtures` feature, `test_fixtures::fixture` are
also read by the world line-of-sight tests in `tools/map_assets/map_asset_verification/src/`.

## Boundaries

- Depends on:
  - the crates that hold every contract type: the blueprint (`building_interiors::blueprint`),
    the compound building and instances (`building_interiors::compound`), the sidecar and BVH
    (`spatial_indexes::bounding_volume_hierarchy`), the descriptors and manifest
    (`world_line_of_sight::occluder_library`) and the archive (`world_file_formats::archives`);
  - the pak reader `enfusion_pak` (`tools/enfusion/enfusion_pak/`), and
    the `repository_root` crate for the checkout root and `repository_layout` for its paths;
  - the schemas in `contracts/definitions/`; the game paks and loose extract, and the
    Workbench exports, dumps, parity files and recon dumps the commands read.
- Used by: the `cargo xtask map` adapters in `tools/xtask/src/commands/map/mod.rs`; the world
  line-of-sight tests in `tools/map_assets/map_asset_verification/src/tests/`, which read
  `parity_report::ParityFile` and `test_fixtures::fixture`.
- Rules:
  - the whole voxel pipeline on the committed farmhouse dump reproduces
    `tools/map_assets/blueprint_compiler/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_blueprint.golden.json`
    (`farmhouse_dump_matches_golden_blueprint`), and that golden blueprint agrees with all 400
    oracle pairs with no phantom blocks (`farmhouse_golden_parity_is_pinned`), both in
    `tests/blueprint_from_voxels_tests.rs`; a heuristic change that moves the output re-blesses the golden on
    purpose;
  - every emitted document passes its schema in `contracts/definitions/` before it is written,
    and the archive refuses inputs that disagree;
  - no file extracted from the game paks is committed; only the derived sidecars, JSON and archive
    are.

## Related documentation

- [Map asset commands](/tools/xtask/src/commands/map/README.md) — the `cargo xtask map`
  commands that run these entries.
- [Everon prefab geometry](/assets/terrains/everon/prefabs/README.md) — the committed output
  and how the map engine loads it.
- [Building interiors](/crates/world_objects/building_interiors/README.md) — the
  blueprint and compound model the output feeds.
- [Blueprint prefab fixtures](/tools/map_assets/blueprint_compiler/test_fixtures/blueprint/prefab/README.md) —
  the `.et` files the resolver tests read.
