# World export pipeline source

The source of the `world_export_pipeline` crate, the library behind the `world` binary: it turns a terrain's
[Workbench](/documentation/glossary/n_to_z.md#workbench) world export and the game's own archives
into the committed object chunks, prefab catalogue, density tiles, forest regions, census, road
network and elevation files under `assets/terrains/<terrain>/`, and runs the gates that prove
those files match their export. The
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map streams what it writes.

## Contents

```text
tools/map_assets/world_export_pipeline/src/
├── binary_emit.rs                the `TBDC` twin of each chunk, read back from its JSON
├── catalog_emit.rs               the `.rkyv` twins of the catalogue, the census and the regions
├── census_counts.rs              the per-bucket prefab and instance counts of a census
├── chunk_partitioner/            the object, density and road builders
├── chunk_partitioner.rs          the builders' module: row types, chunk size, phase order
├── classify.rs                   the prefab classifier and the export line reader
├── command_line.rs               the `world` command line: eighteen subcommands and the exit code
├── empty_write_refusal.rs        `refuse_empty_write`, the guard against empty overwrites
├── enfusion_texture_decoder.rs   the supertexture cell decoder: EDDS, LZ4 blocks, BC7
├── error.rs                      `Error` and `Result`, the context and refusal helpers
├── export_locations.rs           the density fixtures, operation log and type inventory paths
├── export_preparation/           staging, DEM repack, cell index, census, spike and export checks
├── export_preparation.rs         the preparation module: declarations, DEM defaults
├── export_terrain_driver.rs      `cargo xtask map export-terrain`: phase gate, staged export check, object and road builds
├── forest_contours.rs            forest regions derived from tree positions on a 32 m lattice
├── forest_smoothing/             the forest ring smoothing geometry
├── forest_smoothing.rs           the smoothing module: constants, probe and report types
├── json_number_formatting.rs     JSON numbers as JavaScript writes them, rounding, row trailers
├── lib.rs                        the crate root: module header, `mod` lines and the re-exports
├── map_tile_index.rs             `cargo xtask map tile-index`: the map tile pyramid's `index.json`, for the offline pack
├── mathematical_verification/    the `verify-phase` and `phase-gate` gates
├── mathematical_verification.rs  the gates' module: `SchemaSet` and the gate list
├── polygon_geometry.rs           the raw-to-map remap, the chunk partition, the anchor check
├── prelude.rs                    the common names for glob import
├── reclassify/                   the catalogue reclassification body
├── reclassify.rs                 the reclassification module: `Drift`, `Report`, `Mode`
├── roads_emit.rs                 the centrelined `.rkyv` twin of the road JSON
├── tests/                        unit tests for the emitters, builders, decoders, geometry and tile index
├── topo.rs                       the `.topo` road and airfield decoder, with its terrain table
└── vegetation_density.rs         the `TBDD` density grid: corner counts, canopy blur, slicing
```

## How it works

`tools/developer_tools/src/bin/world.rs` calls `entrypoint` (`command_line.rs`), which parses
the subcommand with `clap`, runs it and exits with the code it returns; an error prints
`world: <message>` with its causes and exits 1, and a stage that stops (`Error::Stop`: staged
export files missing, a raw line count or density corner sum that disagrees) prints its message
bare and exits with its own code (2 for missing staging, 1 otherwise). Every path
resolves against the checkout of the working directory (`repository_root::find_repository_root`
walks up from it, and a missing root is an error). An export runs in this order:

```text
Workbench: "Export TBD World Objects (full)" ─▶ profile TBD_WorldExport_full.jsonl + _meta.json
world copy-export-profile --full ─▶ assets/scratch/<terrain>/export/
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

Two modules serve `cargo xtask map` rather than `world`. `export_terrain_driver` runs
`world phase-gate`, then requires `<map scratch dir>/export/raw-entities.jsonl` (else it prints the
Workbench export and staging steps and returns 2), then `world build-objects --patch-manifest
--ops-log` and `world build-roads --ops-log`, each as `cargo run -q -p developer_tools --bin world`
through `process_runner`, printing the child's output when it ends. `map_tile_index` reads the
terrain manifest's `tiles.map.path`, tile extension and zoom range, walks `<z>/<x>/<y>.<extension>`
under the pyramid and writes `index.json` beside it; it is directory and manifest bookkeeping with
no raster code, so xtask runs it in process.

## Public surface

- `entrypoint`: the `world` binary (`tools/developer_tools/src/bin/world.rs`).
- `export_terrain_driver::run` and `map_tile_index::run` (each with a `run_with_root` twin that
  takes the checkout root): xtask's `map` dispatch (`tools/xtask/src/commands/map/dispatch.rs`),
  which exits with the code they return.
- `Error` and `Result`; `prelude` with the entry point, the error and the number and topo readers.
- `export_locations`: the density fixtures, operation log and type inventory paths.
- `json_number_formatting`, `topo::decode_topo` with the `TOPO_*` codes, and
  `enfusion_texture_decoder`: the map raster pipeline
  (`tools/map_assets/map_raster_pipeline/src/`).
- `binary_emit`, `forest_contours`, `polygon_geometry` and `vegetation_density`: the map-object
  golden gate (`tools/map_assets/map_asset_verification/src/object_goldens/`).

## Boundaries

- Depends on: the archives, containers, density codec and POD row of `world_file_formats`
  (`archives`, `containers`, `density`, `pod`); the chunk and manifest readers of `world_chunks`
  and the store of `world_store`; the prefab, region, road and elevation crates
  (`prefab_catalog`, with the census kinds `INSTANCE_KINDS`, `vegetation`, `road_network`,
  `terrain_elevation`); the `enfusion_pak` crate; the `repository_layout` crate and
  `export_locations`;
  `contracts/rules/prefab-classify.json` and the
  schemas in `contracts/definitions/`; `clap`, `serde`, `serde_json`, `jsonschema`, `flate2`,
  `png` and `bcdec_rs`; `cargo`, which `census`, `validate-exports` and the export driver run as a
  child process through `process_runner`.
- Used by: the `world` binary; xtask's `map` dispatch (`cargo xtask map export-terrain` through
  `export_terrain_driver.rs`, `cargo xtask map tile-index` through `map_tile_index.rs`); the
  platform wave gate, which runs `world reclassify`; the map raster pipeline and the map verification gates through the modules above.
- Rules: every census bucket is one of `prefab_catalog`'s `INSTANCE_KINDS`, in its order; a JSON
  artifact and its binary twin change together; no stage writes an empty set over a committed one
  (`refuse_empty_write`); the seven source files that
  `world validate-exports` scans spell no terrain id outside a line marked `E2c-allow` (gate E2c),
  and `topo.rs` holds the per-terrain table; `export-terrain` runs the phase gate before anything
  is built and exits 2 rather than build over a missing staged export; `tile-index` never writes an index over a
  missing or empty pyramid, and what it writes validates against `map-tile-index.schema.json`
  (`tests/map_tile_index/tests.rs`).

## Related documentation

- [Built-in terrains](/assets/terrains/README.md) — the terrain registry and the phases it
  records.
- [Everon world objects](/assets/terrains/everon/objects/README.md) — the files the builders
  write.
- [Map commands](/tools/xtask/src/commands/map/README.md) — `cargo xtask map export-terrain`
  and `cargo xtask map tile-index`, the command lines over the driver and the tile index writer.
- [Developer tool executables](/tools/developer_tools/src/bin/README.md) — the `world` binary.
