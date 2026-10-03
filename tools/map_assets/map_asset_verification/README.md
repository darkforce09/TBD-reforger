# Map asset verification

The `map_asset_verification` crate: the gates over a terrain's committed map assets and the map
golden fixtures that the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
map reads. It checks the terrain manifest, the prefab BLAS library, the height, location, town and
road labels, the elevation anchors and the map-object goldens with the world crates' own readers
and geometry, and probes line of sight through the world occluder over the committed catalogue.

## Contents

```text
tools/map_assets/map_asset_verification/
├── Cargo.toml  the `map_asset_verification` library package: the world crates, `world_export_pipeline`, `jsonschema`, `png`, `flate2`, `thiserror`; layout tier 7
└── src/        the eight gates, the world line-of-sight probe, the error type and their tests
```

## How it works

Each gate takes the checkout root (and a terrain id where it varies), prints one `PASS` or `FAIL`
line per check and returns its exit code; an `Error` means the gate could not run to its verdict,
never that a check failed. The CI task catalogue's map asset checks call the gates for
`cargo xtask schema` (`map-object-golden`, `terrain-manifest`, `height-labels`, `locations`,
`terrain-alignment`, `town-labels`, `road-names`) and `cargo xtask verify blas-manifest`; the
`cargo xtask map world-los` adapter calls the probe. `src/README.md` holds the gate table.

## Getting started

Run from the repository root:

```bash
cargo test -p map_asset_verification                 # the manifest gate cases and the pinned line-of-sight replays
cargo xtask schema terrain-alignment --terrain everon   # one gate over the committed Everon assets
```

The line-of-sight tests and the elevation checks read Git LFS files under
`assets/terrains/everon/` (the prefab catalogue, chunks, descriptors and the DEM); pull those paths
first.

## Boundaries

- Depends on: `world_chunks`, `prefab_catalog`, `world_store`, `world_file_formats`,
  `map_coordinates`, `terrain_elevation`, `place_names`, `label_layout`, `road_network`,
  `spatial_indexes`, `world_line_of_sight` (the readers and geometry the map runs),
  `world_export_pipeline` (the emitters the golden gate checks), `repository_layout` (the contract
  and terrain paths); no async runtime, HTTP client or image codec. Its tests also read
  `blueprint_compiler`'s fixtures through its `test_fixtures` feature (from `[dev-dependencies]`
  only).
- Used by: `tools/xtask/src/commands/schema/dispatch.rs`, `tools/xtask/src/commands/verify/dispatch.rs`,
  `tools/xtask/src/commands/map/mod.rs` and `tools/commands/ci_task_catalog/src/task_definitions/verification_dispatch.rs`.
- Rules: tier 7 of `tools/map_assets`; a gate writes nothing under `assets/` or `contracts/`;
  every gate runs the world crates' functions rather than copies of them, so a gate and the map
  cannot disagree.

## Related documentation

- [Schema commands](/tools/xtask/src/commands/schema/README.md) — the `cargo xtask schema`
  commands that run most of these gates.
- [Map asset commands](/tools/xtask/src/commands/map/README.md) — `cargo xtask map world-los`.
- [Built-in terrains](/assets/terrains/README.md) — the datasets these gates check.
