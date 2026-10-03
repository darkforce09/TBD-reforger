# Developer tools source tree

The source of the `developer_tools` library and its eight executables: offline tooling for
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) archives and scripts, the headless browser gates
of the single-page app, the building-blueprint compiler, the pipelines that build and verify the
terrain and map assets under `assets/`, and the entry points of the staging verification engines.

## Contents

```text
tools/developer_tools/src/
├── bin/                    the eight entry points, one `main` per executable
├── blueprint/              the building-blueprint compiler: mesh decode, voxels, walls, BVH, archives
├── browser_testing/        headless Chromium over the DevTools protocol: gates, smokes, captures
├── lib.rs                  the library root: declares the public modules
├── map_pipeline_layout.rs  the paths only the map pipelines name: density fixtures, export logs, lane records
├── map_raster_pipeline/    map images and archives: orthophoto, satellite, cartographic, labels, water
├── map_verification/       map-asset checks against the map engine: goldens, labels, manifests, sight
├── tests/                  unit tests for the map pipelines' layout
└── world_export_pipeline/  the world-export pipeline behind `world`: objects, roads, density, gates
```

## How it works

Each binary in `bin/` calls one subsystem's command-line entry, and the subsystems share three
foundations: the `enfusion_pak` crate reads the game's archives, the `repository_layout` crate names the
folders and files more than one tool touches, and `repository_layout::find_repository_root`
finds, from the working directory, the checkout root they resolve against. The paths only the
map pipelines name are in `map_pipeline_layout.rs`; the Enfusion oracle and the browser gates
keep theirs in the `enfusion_script_index` crate's `script_index_layout.rs` and `browser_testing/gate_layout.rs`. `map_engine` supplies the formats, geometry and spatial code;
nothing here is compiled for the browser.

```text
bin/gate, bin/capture  ──▶ browser_testing              ──┐
bin/world              ──▶ world_export_pipeline        ──┼──▶ enfusion_pak, repository_layout,
bin/map                ──▶ map_raster_pipeline          ──┤    repository_layout, map_engine
cargo xtask map, schema ─▶ blueprint, map_verification  ──┘

bin/acknowledgement_dropping_relay ──▶ the acknowledgement_dropping_relay crate (tools/staging/acknowledgement_dropping_relay)
bin/staging_load       ──▶ the staging_load_generator crate (tools/staging/staging_load_generator)
bin/mcpd               ──▶ the enfusion_mcp_broker crate (tools/enfusion/enfusion_mcp_broker)
bin/enf                ──▶ the enfusion_script_index crate (tools/enfusion/enfusion_script_index)
```

`blueprint` and `map_verification` have no binary of their own: the `cargo xtask map` and `cargo
xtask schema` commands call their entry functions directly with the checkout root.

## Public surface

- `blueprint`: `run`, `ingest::run`, `parity_report::run` and the `run_*` entries that
  `tools/xtask/src/commands/map/` adapts one to one.
- `map_verification`: `labels`, `object_goldens`, `terrain_manifest`, `blas_manifest` and
  `world_line_of_sight`, which the xtask schema and map verifications call.
- `world_export_pipeline`: `INSTANCE_KINDS`, which the xtask `schema type-inventory` check
  compares against its own copy.
- The eight binaries, whose commands `bin/` lists.

## Boundaries

- Depends on: `map_engine` with its `world` and `streaming` features; the world
  crates `spatial_indexes`, `prefab_catalog`, `world_chunks`, `world_store`,
  `terrain_elevation`, `water_bodies`, `road_network`, `vegetation`, `place_names`,
  `building_interiors`, `interior_line_of_sight`, `world_line_of_sight` and `label_layout`; the image
  crates (`image`, `png`, `image-webp`, `webp`, `resvg`, `bcdec_rs`); `tokio`, `tokio-tungstenite`,
  `axum` and `reqwest` for the browser harness and its servers; `clap`, `serde_json`, `jsonschema`,
  `flate2` and `url`; the foundation crates `repository_layout`, `process_runner`,
  `verification_core`, `time_source` and `content_digest`.
- Used by: `tools/xtask/`, through the modules above (the `map`, `db`, `debug`,
  `generate`, `setup` and `mod` command groups and the `schemas`, `map_assets`, `mod_scripts`
  and `registry` verifications); the CI task `developer-tools-test`; and people, through the
  binaries.
- Rules: the crate never depends on `xtask` (`tooling_dependency_direction_is_enforced`); a
  production source spells a path literal under documentation, .ai, docs or scripts only in
  a layout module (`only_a_layout_module_spells_a_repository_path` in
  `tools/checks/repository_checks/src/tests/tooling_prose_rules.rs`); unit tests live in `tests/` files declared
  with `#[path]`, never inline (`tooling_test_modules_live_in_separate_files`); a production file
  stays under 500 lines, a test file under 1,000, a `bin/` file under 250 and an editor smoke
  scenario under 450 (`tooling_source_files_stay_below_their_structural_limits`, both in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`).
