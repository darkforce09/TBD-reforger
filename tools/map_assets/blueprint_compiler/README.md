# Building-blueprint compiler

The `blueprint_compiler` crate: the offline compiler behind line of sight through buildings and
prefabs in the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator). It reads
[Workbench](/documentation/glossary/n_to_z.md#workbench) voxel dumps and
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) game models, extracts each building's
floors, walls and roof into a blueprint, builds the bounding volume hierarchy (BVH) occlusion
sidecars and the prefab occluder library, folds them into one archive, and checks every step
against Workbench recordings of the engine.

## Contents

```text
tools/map_assets/blueprint_compiler/
├── Cargo.toml      the `blueprint_compiler` library package: the world crates, `enfusion_pak`, `jsonschema`, `flate2`, `thiserror`; layout tier 6
├── src/            the command entries, the voxel interpretation, the model readers, the sidecars and the archive writers
└── test_fixtures/  Workbench recordings, golden compiler outputs and a synthetic prefab tree the unit tests read
```

## How it works

Each `cargo xtask map` blueprint, BVH and model command (`blueprint-from-voxels`,
`ingest-blueprints`, `parity-report`, `voxels-from-mesh`, `bvh-parity`, `bvh-emit`, `bvh-batch`,
`xob-inspect`, `pak-cat`, `instances-verify`, `rotation-pin`) calls one entry of this crate with
the checkout root and its raw arguments; the entry parses them, does the work and returns the exit
code. A failure that stops a command is an `Error`, which xtask prints with its causes.
`src/README.md` holds the two lanes (voxel dumps to blueprints, game models to sidecars) and the
entry table.

## Getting started

Run from the repository root:

```bash
cargo test -p blueprint_compiler   # synthetic buildings, the farmhouse goldens and the committed Everon library
cargo xtask map blueprint-from-voxels --filter FarmHouse   # interpret the matching voxel dumps
```

The tests that read `assets/terrains/everon/prefabs/` and `objects/` need those Git LFS files
pulled; the test that needs a local game extract is ignored.

## Boundaries

- Depends on: `building_interiors`, `spatial_indexes`, `world_line_of_sight`,
  `interior_line_of_sight`, `world_file_formats`, `geometry_primitives` (the contract types it
  builds), `enfusion_pak` (the game paks), `repository_root` (the checkout root),
  `repository_layout` (the contract and terrain paths); no async runtime, HTTP client or image codec.
- Used by: the `cargo xtask map` adapters in `tools/xtask/src/commands/map/mod.rs`; the world
  line-of-sight tests of `map_asset_verification`, through the `test_fixtures` feature
  (from `[dev-dependencies]` only).
- Rules: tier 6 of `tools/map_assets`; every emitted document passes its schema in
  `contracts/definitions/` before it is written; no file extracted from the game paks is
  committed, only the derived sidecars, JSON and archive.

## Related documentation

- [Map asset commands](/tools/xtask/src/commands/map/README.md) — the `cargo xtask map`
  commands that run these entries.
- [Everon prefab geometry](/assets/terrains/everon/prefabs/README.md) — the committed output and
  how the map engine loads it.
- [Building interiors](/crates/world_objects/building_interiors/README.md) — the blueprint and
  compound model the output feeds.
