# Map asset verification

The gates that check a terrain's committed map assets under `assets_v2/terrains/` and the map
golden fixtures under `contracts_v2/fixtures/map/`: the terrain manifest, the prefab BLAS library,
the labels and the elevation model, the map-object goldens, and a line-of-sight probe over the
world occluder. Each runs the map engine's own parsers and geometry, so a passing gate means the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map reads the files as checked.

## Contents

```text
tools_v2/developer-tools/src/map_verification/
├── blas_manifest.rs         the prefab BLAS library: manifest, descriptors, sidecars, hot set
├── labels/                  the label and elevation gate bodies
├── labels.rs                the label gates' module: required towns and roads, re-exports
├── mod.rs                   the module tree
├── object_goldens/          the map-object golden gate, S2 to S9 and S11 to S15
├── object_goldens.rs        the golden gate's module: the `Gate` record, re-exports
├── terrain_manifest.rs      a terrain manifest against its schema, contract and binary blocks
├── tests/                   unit tests for the manifest gate and the pinned line-of-sight replays
├── world_line_of_sight/     the world line-of-sight probe body
└── world_line_of_sight.rs   the probe's module: report and oracle types, elevation sampler
```

## How it works

The folder has no binary. Each gate is a function that takes the checkout root (and a terrain id
where it varies), prints one `PASS` or `FAIL` line per check, and returns its exit code as a
`u8`; `tools_v2/xtask/src/verifications/map_assets/mod.rs` and
`tools_v2/xtask/src/commands/map/mod.rs` adapt each to a command.

| Function | Command | Reads |
|---|---|---|
| `terrain_manifest::terrain_manifest` | `cargo xtask schema terrain-manifest` | `manifest.json`, `terrain-manifest.schema.json`, `map-object-instance.schema.json` |
| `blas_manifest::verify_blas_manifest` | `cargo xtask verify blas-manifest` | Everon's `prefabs/`, `objects/prefabs.json.gz`, the BLAS and descriptor schemas |
| `object_goldens::map_object_golden` | `cargo xtask schema map-object-golden` | `contracts_v2/fixtures/map/` |
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
  commands, through `tools_v2/xtask/src/verifications/map_assets/mod.rs`.
- `world_line_of_sight::run`: `cargo xtask map world-los`.
- `labels::REQUIRED_EVERON_TOWNS`, `labels::MAJOR_EVERON_ROADS`, and the `world_line_of_sight`
  types (`Dem`, `WorldParityFile`, `ReplayReport`) and loaders (`load_cell`, `load_dem`,
  `replay`), are public, but only the gates and their tests use them.

## Boundaries

- Depends on: `website-map-engine` (the `world`, `streaming`, `io` and `bvh` features): its
  manifest, chunk and elevation loaders, label placement, world occluder and container
  constants; the world export pipeline's emitters and geometry
  (`tools_v2/developer-tools/src/world_export_pipeline/`), for the golden gate;
  `crate::repository_layout` for every path; `jsonschema` and the schemas in
  `contracts_v2/definitions/`.
- Used by: `tools_v2/xtask/src/verifications/map_assets/mod.rs` and
  `tools_v2/xtask/src/commands/map/mod.rs`; the CI tasks `schema-validate` (map-object golden and
  height labels), `verify-terrain` and `verify-terrain-strict` in
  `tools_v2/xtask/src/commands/ci/task_definitions.rs`, which check the terrain gates for
  `everon` only.
- Rules: a gate writes nothing under `assets_v2/` or `contracts_v2/`; a path a manifest block
  names that does not exist, and a missing S15 golden binary, are failures, never skips
  (`a_chunks_dir_holding_no_bin_is_dangling`,
  `dangling_binary_paths_are_rejected_one_by_one`); the schema's POD row layout matches the Rust
  POD (`live_pod_row_doc_matches_the_rust_pod`), all in
  `tools_v2/developer-tools/src/map_verification/tests/terrain_manifest.rs`.

## Related documentation

- [Built-in terrains](/assets_v2/terrains/README.md) — the terrain registry and the datasets these
  gates check.
- [Schema commands](/tools_v2/xtask/src/commands/schema/README.md) — the xtask commands that run
  most of these gates.
- [Map fixtures](/contracts_v2/fixtures/map/README.md) — the golden samples.
