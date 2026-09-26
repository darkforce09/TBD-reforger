# Building blueprint compiler

The offline compiler behind line of sight through buildings and prefabs in the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator): it reads voxel dumps and
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) game models, extracts each building's floors,
walls and roof into a blueprint, builds bounding volume hierarchy (BVH) occlusion sidecars and the
prefab occluder library, folds them into one archive, and checks every step against
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) recordings of the engine.

## Contents

```text
tools_v2/developer-tools/src/blueprint/
├── archive_emission/        blueprint assembly, the prefab occluder library and the rkyv archive
├── architectural_analysis/  slabs, walls, floor plates, outlines, roof grid, surface kinds, march skeleton
├── bvh/                     occlusion sidecars, the prefab walk, parity and placement checks
├── ingest.rs                `ingest-blueprints`: Workbench-exported blueprints validated and copied in
├── mesh_decoding/           `.xob` model, collider and node table readers; `xob-inspect` and `pak-cat`
├── mod.rs                   `run` (`blueprint-from-voxels`), per-band assembly, `#[path]` module map
├── parity_report.rs         `parity-report`: the Workbench oracle replayed through a blueprint
├── tests/                   unit tests for decoding, analysis, emission, archives, placement and parity
└── voxel_processing/        the voxel dump model, its reader, the tunables and `voxels-from-mesh`
```

## How it works

`mod.rs` is the module root. Its `#[path]` declarations map short module names (`walls`, `xob`,
`batch`, `archive_emit` and the rest) onto the files of the responsibility folders, and it
re-exports the command entries. Every entry takes the checkout root and the raw arguments from its
`cargo xtask map` adapter, parses them itself and returns the exit code. The compiler has two
lanes that meet in the archive:

```text
Workbench dump action, or voxels-from-mesh
      │  <slug>_voxels.jsonl[.gz]
      ▼
blueprint-from-voxels ──▶ assets_v2/terrains/everon/prefabs/buildings/<slug>.json
                                                    │
game paks ──bvh-batch --all-prefabs──▶ prefabs/blas/, prefabs/descriptors/, blas-manifest.json
      │                                             │
      └──bvh-batch --prefab──▶ buildings/<slug>.bvh, .instances.json, .scene.json
                                                    ▼
                      blueprint-from-voxels archive ──▶ prefabs/building_blueprints.rkyv
```

`run` (`blueprint-from-voxels`) finds `prefabs/dumps/<slug>_voxels.jsonl[.gz]` under the
Workbench profile's `TBD_Export` folder (or `--src`), filters by `--filter`, and interprets each
dump: `slabs::analyze` finds the floors, eave and ridge; `build_bands` extracts walls and a floor
plate for every floor band and adds an attic band of walls when the ridge rises at least
`attic_min_rise_m` above the last band; `emit::assemble` builds the blueprint, and
`validate_and_write` checks it against `contracts_v2/definitions/building-blueprint.schema.json`
and writes it to `--out` or `assets_v2/terrains/everon/prefabs/buildings/`. `--algo` picks the wall
extractor, `--params` overrides tunables, and `--debug-dir` writes a `<slug>_stages.json` with the
slabs and every wall cluster's verdict. A first argument `archive` runs the archive fold instead.

`ingest.rs` copies the blueprints the `tbd-export` building plugins wrote into the Workbench
profile (`prefabs/buildings/*.json`, searched two levels under `TBD_Export`) into
`assets_v2/terrains/everon/prefabs/buildings/`, each only after it parses as `BuildingBlueprint`.
`parity_report.rs` replays a Workbench parity file (engine verdicts for observer and target pairs
in the building's frame, glass excluded) through `BuildingBlueprint::evaluate_los` over a sidecar,
and prints where the model and the engine disagree; it reports and exits 0.

## Public surface

Each entry backs one `cargo xtask map` command (`tools_v2/xtask/src/commands/map/mod.rs`):

| Entry | Command | Exit 0 | Exit 1 |
|---|---|---|---|
| `run` | `blueprint-from-voxels [archive]` | every matched dump written; archive built | no match, a failure |
| `ingest::run` | `ingest-blueprints` | every matched file copied | nothing matched, a file failed |
| `parity_report::run` | `parity-report` | report printed | an argument missing |
| `run_voxels_from_mesh` | `voxels-from-mesh` | dump written, or `--stats` printed | none: errors |
| `run_bvh_parity`, `run_bvh_emit` | `bvh-parity`, `bvh-emit` | report printed; sidecar written | an unknown argument |
| `run_bvh_batch` | `bvh-batch` | files written, or a dry run | an unknown argument |
| `run_xob_inspect`, `run_pak_cat` | `xob-inspect`, `pak-cat` | printed or saved | an unknown argument |
| `run_instances_verify` | `instances-verify` | all within 2 cm and 1° | a mismatch |
| `run_rotation_pin` | `rotation-pin` | `Rigid::from_enfusion` first | another hypothesis first |

Every entry but `run_voxels_from_mesh` also exits 1 on an unknown argument; that one returns an
error instead, and an error any entry returns reaches xtask as a failure.
`parity_report::ParityFile` is also read by the world line-of-sight tests in
`tools_v2/developer-tools/src/map_verification/`.

## Boundaries

- Depends on:
  - `website_map_engine` for every contract type: the blueprint
    (`world::architecture::blueprint`), the compound building and instances
    (`world::architecture::compound`), the sidecar and BVH (`spatial::bvh`), the descriptors and
    manifest (`spatial::los::world::descriptor`) and the archive (`io::archives`);
  - the pak reader in `tools_v2/developer-tools/src/enfusion_pak/`, and
    `crate::repository_layout` and `crate::repository_paths` for the checkout's paths;
  - the schemas in `contracts_v2/definitions/`; the game paks and loose extract, and the
    Workbench exports, dumps, parity files and recon dumps the commands read.
- Used by: the `cargo xtask map` adapters in `tools_v2/xtask/src/commands/map/mod.rs`; the world
  line-of-sight tests in `tools_v2/developer-tools/src/map_verification/tests/`, which read
  `parity_report::ParityFile` and this folder's test fixture helper.
- Rules:
  - the whole voxel pipeline on the committed farmhouse dump reproduces
    `tools_v2/developer-tools/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_blueprint.golden.json`
    (`farmhouse_dump_matches_golden_blueprint`), and that golden blueprint agrees with all 400
    oracle pairs with no phantom blocks (`farmhouse_golden_parity_is_pinned`), both in
    `tests/module/tests.rs`; a heuristic change that moves the output re-blesses the golden on
    purpose;
  - every emitted document passes its schema in `contracts_v2/definitions/` before it is written,
    and the archive refuses inputs that disagree;
  - no file extracted from the game paks is committed; only the derived sidecars, JSON and archive
    are.

## Related documentation

- [Map asset commands](/tools_v2/xtask/src/commands/map/README.md) — the `cargo xtask map`
  commands that run these entries.
- [Everon prefab geometry](/assets_v2/terrains/everon/prefabs/README.md) — the committed output
  and how the map engine loads it.
- [Building architecture](/apps/website/map-engine/src/world/architecture/README.md) — the
  blueprint and compound model the output feeds.
- [Blueprint prefab fixtures](/tools_v2/developer-tools/test_fixtures/blueprint/prefab/README.md) —
  the `.et` files the resolver tests read.
