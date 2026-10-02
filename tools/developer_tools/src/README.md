# Developer tools source tree

The source of the `developer_tools` library and its seven executables: offline tooling for
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) archives and scripts, the headless browser gates
of the single-page app, the building-blueprint compiler, the pipelines that build and verify the
terrain and map assets under `assets/`, and the engines of the staging verification receipts.

## Contents

```text
tools/developer_tools/src/
├── bin/                     the seven entry points, one `main` per executable
├── blueprint/               the building-blueprint compiler: mesh decode, voxels, walls, BVH, archives
├── browser_testing/         headless Chromium over the DevTools protocol: gates, smokes, captures
├── content_digest.rs        SHA-384 of a file's bytes, in the hex spelling `sqlx` stores for migrations
├── enfusion_pak/            the game's `.pak` archives and loose folders behind one virtual file system
├── enfusion_tooling/        the Enfusion script oracle behind `enf` and the enfusion-mcp broker
├── lib.rs                   the library root: declares the twelve public modules
├── map_raster_pipeline/     map images and archives: orthophoto, satellite, cartographic, labels, water
├── map_verification/        map-asset checks against the map engine: goldens, labels, manifests, sight
├── repository_layout.rs     every repository path the crate spells, as constants and path functions
├── repository_paths.rs      checkout discovery: walks up to the ticket registry's root marker
├── staging_verification/    engines the staging harness runs against the staging host: the member load, the relay
├── tests/                   unit tests for the digest, the layout and the checkout discovery
├── timestamp_formatting.rs  UTC ISO-8601 timestamps with milliseconds for the emitted artifacts
└── world_export_pipeline/   the world-export pipeline behind `world`: objects, roads, density, gates
```

## How it works

Each binary in `bin/` calls one subsystem's command-line entry, and the subsystems share three
foundations: `enfusion_pak` reads the game's archives, `repository_layout` names every folder and
file they touch, and `repository_paths` (or `browser_testing::server::repo_root`) finds the checkout
root they resolve against. `map_engine` supplies the formats, geometry and spatial code;
nothing here is compiled for the browser.

```text
bin/enf, bin/mcpd      ──▶ enfusion_tooling             ──┐
bin/gate, bin/capture  ──▶ browser_testing              ──┤
bin/world              ──▶ world_export_pipeline        ──┼──▶ enfusion_pak, repository_layout,
bin/map                ──▶ map_raster_pipeline          ──┤    repository_paths, map_engine
cargo xtask map, schema ─▶ blueprint, map_verification  ──┘

bin/acknowledgement_dropping_relay ──▶ staging_verification::acknowledgement_relay
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
- `enfusion_tooling::enfusion_mcp_entrypoint`: the enfusion-mcp server command that `cargo xtask mcp
  call` and `cargo xtask mcp daemon` start.
- `repository_layout`: the contract, fixture, terrain, glyph and MCP package paths that xtask
  commands and verifications resolve, `mission_fixtures_valid_dir` among them.
- `content_digest`: the migration checksum `cargo xtask db repair-migration-checksum` computes.
- `staging_verification::load_generation`: `run` and its plan and report types, the member load
  engine behind the staging load receipt.
- `staging_verification::acknowledgement_relay`: `entrypoint`, `start`, `serve` and the
  `RelayStatus` document `control` prints, the relay behind the staging fleet receipt's
  lost-acknowledgement cases.
- The seven binaries, whose commands `bin/` lists.

## Boundaries

- Depends on: `map_engine` with its `world`, `streaming`, `io` and `bvh` features; the image
  crates (`image`, `png`, `image-webp`, `webp`, `resvg`, `bcdec_rs`); `tokio`, `tokio-tungstenite`,
  `axum` and `reqwest` for the browser harness and its servers; `clap`, `serde_json`, `jsonschema`,
  `flate2` and `sha2`.
- Used by: `tools/xtask/`, through the modules above (the `map`, `mcp`, `db`, `debug`,
  `generate`, `setup` and `mod_ops` command groups and the `schemas`, `map_assets`, `mod_scripts`
  and `registry` verifications); the CI task `developer-tools-test`; and people, through the
  binaries.
- Rules: the crate never depends on `xtask` (`tooling_dependency_direction_is_enforced`); a
  production source spells a path literal under documentation, .ai, docs or scripts only in
  `repository_layout.rs` (`only_a_layout_module_spells_a_repository_path` in
  `tools/xtask/src/tests/tooling_prose_rules.rs`); unit tests live in `tests/` files declared
  with `#[path]`, never inline (`tooling_test_modules_live_in_separate_files`); a production file
  stays under 500 lines, a test file under 1,000, a `bin/` file under 250 and an editor smoke
  scenario under 450 (`tooling_source_files_stay_below_their_structural_limits`, both in
  `tools/xtask/src/tests/tooling_dependency_boundaries.rs`).
