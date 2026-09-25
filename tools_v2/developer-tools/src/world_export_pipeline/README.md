# World export pipeline

The library behind the `world` binary: it turns a terrain's
[Workbench](/documentation_v2/glossary.md#workbench) world export and the game's own archives
into the committed object chunks, prefab catalogue, density tiles, forest regions, census, road
network and elevation files under `assets_v2/terrains/<terrain>/`, and runs the gates that prove
those files match their export. The
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map streams what it writes.

## Contents

```text
tools_v2/developer-tools/src/world_export_pipeline/
├── binary_emit.rs                the `TBDC` twin of each chunk, read back from its JSON
├── catalog_emit.rs               the `.rkyv` twins of the catalogue, the census and the regions
├── chunk_partitioner/            the object, density and road builders
├── chunk_partitioner.rs          the builders' module: row types, chunk size, phase order
├── classify.rs                   the prefab classifier and the export line reader
├── cli.rs                        the `world` command line: eighteen subcommands
├── enfusion_texture_decoder.rs   the supertexture cell decoder: EDDS, LZ4 blocks, BC7
├── export_preparation/           staging, DEM repack, cell index, census, spike and export checks
├── export_preparation.rs         the preparation module: declarations, DEM defaults
├── forest_contours.rs            forest regions derived from tree positions on a 32 m lattice
├── forest_smoothing/             the forest ring smoothing geometry
├── forest_smoothing.rs           the smoothing module: constants, probe and report types
├── json_number_formatting.rs     JSON numbers as JavaScript writes them, rounding, row trailers
├── mathematical_verification/    the `verify-phase` and `phase-gate` gates
├── mathematical_verification.rs  the gates' module: `SchemaSet` and the gate list
├── mod.rs                        the module tree; `INSTANCE_KINDS` and `refuse_empty_write`
├── polygon_geometry.rs           the raw-to-map remap, the chunk partition, the anchor check
├── reclassify/                   the catalogue reclassification body
├── reclassify.rs                 the reclassification module: `Drift`, `Report`, `Mode`
├── roads_emit.rs                 the centrelined `.rkyv` twin of the road JSON
├── tests/                        unit tests for the emitters, builders, decoders and geometry
├── topo.rs                       the `.topo` road and airfield decoder, with its terrain table
└── vegetation_density.rs         the `TBDD` density grid: corner counts, canopy blur, slicing
```

## How it works

`tools_v2/developer-tools/src/bin/world.rs` calls `cli::entrypoint`, which parses the
subcommand with `clap`, runs it and exits with the code it returns; an error prints
`world: <message>` and exits 1. Every path
resolves against the checkout the binary was compiled in (`browser_testing::server::repo_root`
reads `CARGO_MANIFEST_DIR`). An export runs in this order:

```text
Workbench: "Export TBD World Objects (full)" ─▶ profile TBD_WorldExport_full.jsonl + _meta.json
world copy-export-profile --full ─▶ assets_v2/scratch/<terrain>/export/
cargo xtask map export-terrain <terrain> --phase <P> ─▶ world phase-gate ─▶ world build-objects
                                                        ─▶ world build-roads
world verify-phase ─▶ the importPhaseMax bump in terrain-registry.json, by hand
```

| Subcommand | Module | Does |
|---|---|---|
| `copy-export-profile`, `raw-u16-dem-png`, `sap-catalog`, `census`, `spike-k1`, `spike-census`, `spike-ops-log`, `validate-exports` | `export_preparation` | stage an export, pack the DEM, index cells, census and check the artifacts |
| `build-objects`, `redensify`, `gen-density-fixture`, `build-roads` | `chunk_partitioner` | write chunks, catalogue, density, regions, census; rebuild density; write roads |
| `roads-rkyv` | `roads_emit` | rewrite `roads/road_network.rkyv` from the committed road JSON |
| `reclassify` | `reclassify` | report or apply classification drift in the committed catalogue |
| `phase-gate`, `verify-phase` | `mathematical_verification` | check the registry phase; prove the committed artifacts |
| `topo-stats` | `topo` | print the `.topo` file's sections and a record histogram |
| `edds-cell <N>` | `enfusion_texture_decoder` | write one supertexture cell as raw RGBA to standard output |

`build-objects`, `build-roads`, `topo-stats`, `edds-cell`, `sap-catalog` and the E6 gate of
`verify-phase` read the game's archives through `enfusion_pak::PakVfs`; `redensify`,
`roads-rkyv`, `reclassify` and `validate-exports` need only the checkout. Each JSON artifact is
written first and its binary twin is built by reading that JSON back through the map engine's own
loader (`binary_emit`, `catalog_emit`, `roads_emit`), so both forms decode to the same rows. Every
number goes through `json_number_formatting::js_num`, so a rebuild is byte-comparable with the
committed file, and `refuse_empty_write` stops any stage from replacing committed data with an
empty set. `polygon_geometry` re-implements the remap and partition on purpose, so the gates do
not certify the builder with its own code.

## Public surface

- `cli::entrypoint`: the `world` binary (`tools_v2/developer-tools/src/bin/world.rs`).
- `INSTANCE_KINDS`: the census kinds, compared with the copy in
  `tools_v2/xtask/src/verifications/schemas/checks.rs` by
  `tools_v2/xtask/src/verifications/schemas/checks/object_type_inventory.rs`.
- `json_number_formatting`, `topo::decode_topo` with the `TOPO_*` codes, and
  `enfusion_texture_decoder`: the map raster pipeline
  (`tools_v2/developer-tools/src/map_raster_pipeline/`).
- `binary_emit`, `forest_contours`, `polygon_geometry` and `vegetation_density`: the map-object
  golden gate (`tools_v2/developer-tools/src/map_verification/object_goldens/`).

## Boundaries

- Depends on: `website-map-engine`'s archives, containers, density codec and POD row
  (`io::archives`, `io::containers`, `io::density`, `io::pod`), its loaders
  (`streaming::loaders`) and its prefab, region, road and elevation code (`world::environment`,
  `world::terrain`); `crate::enfusion_pak`; `crate::repository_layout`;
  `crate::browser_testing::server::repo_root`; `contracts_v2/rules/prefab-classify.json` and the
  schemas in `contracts_v2/definitions/`; `clap`, `serde_json`, `jsonschema`, `flate2`, `png` and
  `bcdec_rs`; `cargo`, which `census` and `validate-exports` run as a child process.
- Used by: the `world` binary; `cargo xtask map export-terrain`
  (`tools_v2/xtask/src/commands/map/terrain_export.rs`); the platform wave gate, which runs `world
  reclassify`; the map raster pipeline and the map verification gates through the modules above;
  the xtask census-kind check.
- Rules: `INSTANCE_KINDS` equals the schema's kind enum minus the region kinds
  (`instance_kinds_match_enums_schema` in `tests/module/instance_kind_tests.rs`); a JSON artifact
  and its binary twin change together; no stage writes an empty set over a committed one
  (`refuse_empty_write`, `tests/module/refuse_empty_tests.rs`); the seven source files that
  `world validate-exports` scans spell no terrain id outside a line marked `E2c-allow` (gate E2c),
  and `topo.rs` holds the per-terrain table.

## Related documentation

- [Built-in terrains](/assets_v2/terrains/README.md) — the terrain registry and the phases it
  records.
- [Everon world objects](/assets_v2/terrains/everon/objects/README.md) — the files the builders
  write.
- [Map commands](/tools_v2/xtask/src/commands/map/README.md) — `cargo xtask map export-terrain`.
- [Developer tool executables](/tools_v2/developer-tools/src/bin/README.md) — the `world` binary.
