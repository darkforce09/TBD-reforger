# Map raster pipeline

The `map_raster_pipeline` crate: it turns the game's archives and the
[Workbench](/documentation/glossary/n_to_z.md#workbench) exports into the image and label assets a
terrain serves under `assets/terrains/<terrain>/` (the unified satellite container, the satellite
and Map view tile pyramids, the label sets and archives, the water archives) and the world-glyph
atlas under `assets/glyphs/atlas/`, and verifies each against the terrain's `manifest.json`. The
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map shows what it writes.
It also draws a Workbench water or road export as PNG images for inspection, into a folder beside
the export.
The `map` binary of `developer_tools` is its command line.

## Contents

```text
tools/map_assets/map_raster_pipeline/
├── Cargo.toml  the `map_raster_pipeline` library package: `world_export_pipeline`, the world format, terrain and place-name crates, `grid_rasterization`, `enfusion_pak`, `clap`, `serde_json`, `image`, `png`, `image-webp`, `webp`, `resvg`, `thiserror`; layout tier 7
└── src/        the orthophoto, satellite, cartographic, label, water, glyph and export image lanes and the `map` command line
```

## How it works

```text
game paks ──▶ map stitch-sap-ortho ──▶ assets/scratch/everon/sap/ ──▶ map analyze-water, composite-water
                                                                   ──▶ map build-landcover, build-cartographic
source PNG ──▶ map build-unified ──▶ satellite/<terrain>-sat.tbd-sat   map build-pyramid ──▶ tiles/<view>/
raw entity export, elevation model ──▶ map export-locations, export-height-labels ──▶ map labels-rkyv
Workbench inland-water export ──▶ map water ──▶ water/water_vectors.rkyv, water/bathymetry.tbd-bath
assets/glyphs/svg/ ──▶ map build-glyph-atlas ──▶ assets/glyphs/atlas/
Workbench water export folder ──▶ map water-images ──▶ <water folder>/images/<terrain>-water-*.png
Workbench road export folder ──▶ map road-images ──▶ <roads folder>/images/<terrain>-roads-*.png, layer-*.png
```

Every asset lane refuses an empty result rather than overwrite a committed asset, reads each archive
back through its validating reader before writing it, and gives the same bytes for the same
inputs. The two export image lanes take their export folder on the command line and write only
into their image folder. `src/README.md` holds the subcommand table and each lane.

## Getting started

Run from the repository root:

```bash
cargo test -p map_raster_pipeline   # the archives, label exporters, container and locations
cargo run -q -p developer_tools --bin map -- --help
cargo run -q -p developer_tools --bin map -- water-images <export folder> --mode all
cargo run -q -p developer_tools --bin map -- road-images <export folder>/everon/roads
```

The Everon label tests read the committed label files under `assets/terrains/everon/`; the
satellite and DEM lanes read Git LFS objects there (`git lfs pull --include "assets/terrains/everon/**"`).

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `ENFUSION_GAME_PATH` | `<checkout>/.workstation/enfusion_mcp_game_root` | the game folder whose archives `stitch-sap-ortho`, `analyze-water` and `build-cartographic` read (through `enfusion_pak`) |

## Boundaries

- Depends on: `world_export_pipeline` (the number spelling, the `.topo` road decoder, the texture
  decoder), `world_file_formats`, `terrain_elevation`, `water_bodies` (with the bathymetry
  palette), `road_network` (with the export image styles), `grid_rasterization`,
  `prefab_catalog`, `place_names`, `enfusion_pak`, `repository_layout`, `process_runner`,
  `time_source`, `content_digest`; `clap`, `serde_json`, `image`, `png`, `image-webp`, `webp`,
  `resvg`, `thiserror`.
- Used by: the `map` binary of `developer_tools` (`entrypoint`); the `map-water-everon`,
  `map-cartographic-everon` and `map-cartographic-verify` tasks of `cargo xtask ci`, which run that
  binary as a child process.
- Rules: tier 7 of `tools/map_assets` (`cargo xtask verify crate-tiers`); never in xtask's
  dependency closure, so the image codecs stay out of it; every failure is an `Error` and only the
  `map` binary decides the exit code; `verify-cartographic` runs `cargo xtask schema` and the
  `world` binary as child processes through `process_runner`.

## Related documentation

- [Map raster pipeline](/documentation/tools/map_assets/map_raster_pipeline.md) — the satellite,
  Map view, label, water, glyph and export image lanes in depth, with their rules and open work.
- [Everon terrain assets](/assets/terrains/everon/README.md) — the committed files these lanes
  write.
- [Developer tool executables](/tools/developer_tools/src/bin/README.md) — the `map` binary.
