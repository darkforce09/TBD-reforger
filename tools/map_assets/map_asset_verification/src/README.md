# Map asset verification sources

The gates that check a terrain's committed map assets under `assets/terrains/` and the map
golden fixtures under `contracts/fixtures/map/`: the terrain manifest, the prefab BLAS library,
the labels and the elevation model, the map-object goldens, and a line-of-sight probe over the
world occluder. Each runs the map engine's own parsers and geometry, so a passing gate means the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map reads the files as checked.

## Contents

```text
tools/map_assets/map_asset_verification/src/
├── blas_manifest.rs         the prefab BLAS library: manifest, descriptors, sidecars, hot set
├── error.rs                 `Error`: a step and its cause chain, or a wrapped library error
├── labels/                  the label and elevation gate bodies
├── labels.rs                the label gates' module: required towns and roads, re-exports
├── lib.rs                   the crate root: the module tree and the error re-export
├── object_goldens/          the map-object golden gate, S2 to S9 and S11 to S15
├── object_goldens.rs        the golden gate's module: the `Gate` record, re-exports
├── prelude.rs               the error type and the eight gate entries, for the adapters
├── terrain_manifest.rs      a terrain manifest against its schema, contract and binary blocks
├── tests/                   unit tests for the manifest gate and the pinned line-of-sight replays
├── world_line_of_sight/     the world line-of-sight probe body
└── world_line_of_sight.rs   the probe's module: report and oracle types, elevation sampler
```

## How it works

The crate has no binary. Each gate is a function that takes the checkout root (and a terrain id
where it varies), prints one `PASS` or `FAIL` line per check, and returns its exit code as a
`u8`; an `Err` means the gate could not run to its verdict (an input it needs did not read or
parse, a schema did not compile), never that a check failed; the xtask `schema`, `verify` and
`map` dispatch and `tools/commands/ci_task_catalog/src/task_definitions/verification_dispatch.rs` call each with the repository root.

| Function | Command | Reads |
|---|---|---|
| `terrain_manifest::terrain_manifest` | `cargo xtask schema terrain-manifest` | `manifest.json`, `terrain-manifest.schema.json`, `map-object-instance.schema.json` |
| `blas_manifest::verify_blas_manifest` | `cargo xtask verify blas-manifest` | Everon's `prefabs/`, `objects/prefabs.json.gz`, the BLAS and descriptor schemas |
| `object_goldens::map_object_golden` | `cargo xtask schema map-object-golden` | `contracts/fixtures/map/` |
| `labels::height_labels`, `locations`, `town_labels`, `road_names`, `terrain_alignment` | `cargo xtask schema height-labels`, `locations`, `town-labels`, `road-names`, `terrain-alignment` | the terrain's label files, anchors and DEM |
| `world_line_of_sight::run` | `cargo xtask map world-los` | a cell's chunks, descriptors and BLAS sidecars |

`terrain_manifest` checks the manifest against its schema, then against a contract compiled into
the gate for `everon` (12,800 m, heights -204.78 to 375.53 m) and `arland` (4,096 m, -163.0 to
148.38 m), exiting 2 for any other id. It then checks the binary blocks: the `objects.binary`
container, row type and size must match what this build of the map engine reads, the chunk path
template must carry both `{cx}` and `{cy}` and resolve to a folder holding `.bin` files, every path
a block names must exist, and a key that is present but empty fails. It also checks that the
`objectInstancePodRow` layout in `map-object-instance.schema.json` covers `POD_BYTES` exactly.

`verify_blas_manifest` checks Everon only: `blas-manifest.json` validates and its lists are
sorted, every catalogue prefab has a valid descriptor whose BLAS paths are in the manifest, every
BLAS file has the manifest's byte size and triangle and kind counts, the hot set lists blocking
prefabs most-placed first, and the farmhouse root BLAS equals the committed shell sidecar. It
reads the files the manifest lists and never the folder, so a `.bvh` the manifest omits goes
unreported.

## Public surface

- `terrain_manifest::terrain_manifest`, `blas_manifest::verify_blas_manifest`,
  `object_goldens::map_object_golden` and the five `labels` gates: the xtask schema and verify
  commands, called from `tools/xtask/src/commands/schema/dispatch.rs` and
  `tools/xtask/src/commands/verify/dispatch.rs`.
- `world_line_of_sight::run`: `cargo xtask map world-los`.
- `Error` and `Result`: an `Error::Context` displays its step alone and carries the failure
  underneath as its source, so `xtask: …` prints the step and every cause, and a gate that
  prints a read failure itself (`terrain_manifest`) prints the step alone.
- `labels::REQUIRED_EVERON_TOWNS`, `labels::MAJOR_EVERON_ROADS`, and the `world_line_of_sight`
  types (`Dem`, `WorldParityFile`, `ReplayReport`) and loaders (`load_cell`, `load_dem`,
  `replay`), are public, but only the gates and their tests use them.

## Boundaries

- Depends on: the world crates it imports directly: `world_chunks` (the manifest and chunk
  readers), `prefab_catalog`, `world_store`, `world_file_formats`, `map_coordinates`,
  `terrain_elevation` (the elevation model), `place_names` and `label_layout` (label placement),
  `road_network`, `spatial_indexes` and `world_line_of_sight` (the world occluder); the world
  export pipeline's emitters and geometry (`tools/map_assets/world_export_pipeline/src/`), for
  the golden gate; the `repository_layout` crate for every path; `jsonschema` and the schemas in
  `contracts/definitions/`; `flate2`, `png`, `regex`, `serde_json` and `thiserror`. Its tests
  also read `blueprint_compiler`'s world-parity fixtures, `building_interiors` and
  `geometry_primitives`.
- Used by: `tools/xtask/src/commands/schema/dispatch.rs`, `tools/xtask/src/commands/verify/dispatch.rs` and
  `tools/xtask/src/commands/map/mod.rs`; the CI tasks `schema-validate` (map-object golden and
  height labels), `verify-terrain` and `verify-terrain-strict` in
  `tools/commands/ci_task_catalog/src/task_definitions.rs` (through
  `task_definitions/verification_dispatch.rs`), which check the terrain gates for
  `everon` only.
- Rules: a gate writes nothing under `assets/` or `contracts/`; a path a manifest block
  names that does not exist, and a missing S15 golden binary, are failures, never skips
  (`a_chunks_dir_holding_no_bin_is_dangling`,
  `dangling_binary_paths_are_rejected_one_by_one`); the schema's POD row layout matches the Rust
  POD (`live_pod_row_doc_matches_the_rust_pod`), all in
  `tools/map_assets/map_asset_verification/src/tests/terrain_manifest.rs`.

## Related documentation

- [Built-in terrains](/assets/terrains/README.md) — the terrain registry and the datasets these
  gates check.
- [Schema commands](/tools/xtask/src/commands/schema/README.md) — the xtask commands that run
  most of these gates.
- [Map fixtures](/contracts/fixtures/map/README.md) — the golden samples.
