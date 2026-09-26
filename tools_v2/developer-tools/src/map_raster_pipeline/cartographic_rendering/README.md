# Cartographic map rendering

The stylised Map view of a terrain: land-cover masks classified from the stitched orthophoto, the
cartographic image drawn from the [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) satellite
export with tints, inland water and roads, the WebP tile pyramid built from it, and the manifest
patches and checks around them. These files are the submodules
`tools_v2/developer-tools/src/map_raster_pipeline/cartographic_rendering.rs` declares; it holds the
land-cover thresholds and tint colours, and re-exports the entry points.

## Contents

```text
tools_v2/developer-tools/src/map_raster_pipeline/cartographic_rendering/
├── build_landcover_masks.rs  the land-cover masks, and the cartographic render with its road strokes
└── build_tile_pyramid.rs     the WebP tile pyramid, three manifest patches, the cartographic check
```

## How it works

`cargo xtask ci map-cartographic-everon` runs `build-cartographic`, then `build-pyramid` into the
terrain's `tiles/map/`, then `patch-map-tiles-meta`, then `verify-pyramid --view-map`.

```text
scratch/everon/sap/everon-sap-ortho.png ─▶ build_landcover_masks (3200² classes, blur-threshold)
                                            ─▶ scratch/everon/map/landcover-*-mask.png + meta
scratch/everon/spike/TBD_SatExport_everon.tga (4096²)
  ─▶ open tint over the bright mask, forest tint over the forest mask ─▶ Lanczos upscale to 12800²
  ─▶ inland-water tint (scratch/everon/sap/water-inland-mask.png, when present)
  ─▶ road polylines from the game's .topo file, rasterised with resvg
  ─▶ scratch/everon/map/everon-map-ortho.png + map-ortho-meta.json
```

- `build_landcover_masks` samples the orthophoto at `CLASS_PX` (3,200 px), classes each pixel as
  water (blue at least green), forest, bright ground or grass by the thresholds in the parent file,
  cleans each mask with box-blur thresholds, and writes soft-edged masks and their fractions. Only
  `everon` has a source; any other terrain is an error.
- `build_map_cartographic` decodes the terrain's `.topo` file through
  `crate::world_export_pipeline::topo::decode_topo`, strips spikes from each road polyline
  (`despike`) and strokes each road type in the colour and width `road_style` gives it.
- `build_tile_pyramid` resizes the input to each zoom level, cuts `z/x/y.webp` tiles (lossless, or
  lossy at `--quality`), optionally flips the image vertically, writes `full.webp` at no more than
  4,096 px a side, and fails when `0/0/0.webp` is missing afterwards.
- `reset_water_meta` and `patch_unified_bytes` are steps of `cargo xtask ci map-water-everon`: the
  first drops the `waterComposite` block from the orthophoto metadata, the second writes the
  satellite container's byte size into the manifest. `patch_map_tiles_meta` sets the manifest's
  `tiles.map` source and encoding (`workbench-cartographic`, `webp-lossy`).
- `verify_cartographic` requires ten slice verification logs under `.ai/artifacts/` to hold a pass
  verdict, then runs `cargo xtask schema map-glyphs`, `world validate-exports`, `world verify-phase
  --phase P5_props` and the `cargo xtask schema` locations, height-label, town-label and road-name
  checks for Everon, and fails if any of them fails.

## Boundaries

- Depends on: `super::image_operations`; `crate::enfusion_pak::PakVfs` and
  `crate::world_export_pipeline::topo` for the road geometry; `resvg` for the strokes;
  `crate::repository_layout` for the scratch, terrain and artifact folders;
  `crate::browser_testing::server::repo_root` for the checkout root.
- Used by: `tools_v2/developer-tools/src/map_raster_pipeline/cli.rs` (the `build-landcover`,
  `build-cartographic`, `build-pyramid`, `reset-water-meta`, `patch-unified-bytes`,
  `patch-map-tiles-meta` and `verify-cartographic` subcommands); the `map-water-everon` and
  `map-cartographic-everon` tasks in `tools_v2/xtask/src/commands/ci/task_definitions.rs`.
- Rules: the pyramid lands in the terrain's `tiles/` folder, which git ignores
  (`assets_v2/terrains/**/tiles/` in `.gitignore`), while the manifest patches change the committed
  `manifest.json`; the land-cover thresholds and tint colours are named constants in
  `cartographic_rendering.rs`, and the cartographic render refuses a base image that is not 4,096 px
  square or a water mask that is not 12,800 px square.
