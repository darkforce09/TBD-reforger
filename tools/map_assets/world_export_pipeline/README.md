# World export pipeline

The `world_export_pipeline` crate: it turns a terrain's
[Workbench](/documentation/glossary/n_to_z.md#workbench) world export and the game's own archives
into the committed object chunks, prefab catalogue, census, density tiles, forest regions, road
network and elevation files under `assets/terrains/<terrain>/`, and runs the gates that prove those
files match their export. The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
map streams what it writes. The `world` binary of `developer_tools` is its command line. It also
holds the `cargo xtask map export-terrain` driver, which runs the export stages as `world` child
processes, and the `cargo xtask map tile-index` writer, which lists a terrain's map tile pyramid
for the offline pack.

## Contents

```text
tools/map_assets/world_export_pipeline/
├── Cargo.toml  the `world_export_pipeline` library package: the world format, terrain and world object crates, `enfusion_pak`, `clap`, `serde`, `serde_json`, `jsonschema`, `flate2`, `png`, `bcdec_rs`, `thiserror`; layout tier 6
└── src/        the stages, the emitters, the decoders, the gates, the `world` command line, the export driver and the tile index writer
```

## How it works

```text
Workbench export ──▶ world copy-export-profile ──▶ assets/scratch/<terrain>/export/
cargo xtask map export-terrain ──▶ world phase-gate ──▶ world build-objects ──▶ world build-roads
                                                         │                       │
                                                         ▼                       ▼
                       objects/ chunks, catalogue, census, density, regions    objects/roads.json.gz, roads/road_network.rkyv
world verify-phase ──▶ the gates over the committed artifacts
cargo xtask map tile-index --terrain <terrain> ──▶ assets/terrains/<terrain>/tiles/map/index.json
```

Every JSON artifact is written first and its binary twin is built by reading that JSON back, so
both forms decode to the same rows; every number is written as JavaScript writes it, so a rebuild
is byte-comparable with the committed file; no stage writes an empty set over committed data. The
census buckets come from `prefab_catalog::instance_kinds::INSTANCE_KINDS`, the list the
type-inventory gate of `schema_tooling` sums. The tile index writer is directory and manifest
bookkeeping with no raster code, so xtask runs it in process; the raster pipeline that builds the
pyramid stays out of xtask's dependency closure. `src/README.md` holds the subcommand table and each
module.

## Getting started

Run from the repository root:

```bash
cargo test -p world_export_pipeline   # the emitters, decoders, geometry and tile index
cargo run -q -p developer_tools --bin world -- --help
```

The Everon tests read Git LFS objects under `assets/terrains/everon/`; pull them first
(`git lfs pull --include "assets/terrains/everon/**"`).

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `ENFUSION_GAME_PATH` | `<checkout>/.workstation/enfusion_mcp_game_root` | the game folder whose archives `build-objects`, `build-roads`, `topo-stats`, `edds-cell`, `sap-catalog` and the E6 gate read (through `enfusion_pak`) |
| `PROFILE`, `ENFUSION_PROFILE_PATH` | `$HOME/Documents/Games/ArmaReforgerWorkbench/profile` | the [Workbench](/documentation/glossary/n_to_z.md#workbench) profile folder `copy-export-profile` reads when `--profile` is not given, `PROFILE` first |

## Boundaries

- Depends on: `world_file_formats`, `prefab_catalog`, `world_chunks`, `world_store`,
  `terrain_elevation`, `road_network`, `vegetation`, `enfusion_pak`, `repository_layout`,
  `process_runner`, `verification_core`, `time_source`; `clap`, `serde`, `serde_json`,
  `jsonschema`, `flate2`, `png`, `bcdec_rs`, `thiserror`; `deterministic_random` (tests only).
- Used by: the `world` binary of `developer_tools` (`entrypoint`); the `map_raster_pipeline`
  crate (`json_number_formatting`, `topo`, `enfusion_texture_decoder`) and the map verification
  (`binary_emit`, `forest_contours`, `polygon_geometry`, `vegetation_density`); xtask's `map`
  dispatch (`export_terrain_driver::run`, `map_tile_index::run`).
- Rules: tier 6 of `tools/map_assets` (`cargo xtask verify crate-tiers`); no tokio, axum, reqwest,
  resvg or image in its dependency tree, since xtask depends on it; every failure is an `Error`
  and only the binary (`world` or xtask) decides the exit code; `census` and `validate-exports` run `cargo xtask schema type-inventory`
  and `cargo xtask map export-terrain` as child processes through `process_runner`.

## Related documentation

- [Terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md) — the
  export run end to end.
- [Built-in terrains](/assets/terrains/README.md) — the terrain registry and the phases it
  records.
- [Developer tool executables](/tools/developer_tools/src/bin/README.md) — the `world` binary.
