# World layers on the GPU

The `world_layers_gpu` crate: the typed GPU layers of the streamed world the map draws.
`BuildingLayerGpu` uploads the building footprints, outlines and fence strips as lanes;
`ForestLayerGpu` shows the forest mass's density raster as the textured forest fill lane and
applies the forest lane settings; `TerrainTextureLayerGpu` begins, writes and commits the
satellite basemap and the hillshade as texture lanes; `TerrainLineOfSightOverlayGpu` shows a
finished viewshed raster of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
line-of-sight tool as the viewshed lane. The textured lanes share `TexLane`, the texture record the
renderer keeps beside a lane's batch. Every lane write goes through `renderer_core`'s `LaneSink`.

## Contents

```text
crates/map_rendering/world_layers_gpu/
├── Cargo.toml  the package: `renderer_core`, `gpu_frame`, `render_primitives`, `map_draw_lanes`, `map_coordinates`; `wgpu` and `web-sys` on wasm32; layout tier 3, wasm32
└── src/        the typed layers, the textured lane record, the basemap mode, the textured quad and raster rules, the error type and their tests
```

## How it works

```text
map_asset_loading loaders ─▶ renderer's asset sink ─▶ BuildingLayerGpu::{upload_buildings, upload_outlines, upload_fence_strips}
                                                    ├▶ ForestLayerGpu::{upload_density, set_params, set_outline_stored}
                                                    └▶ TerrainTextureLayerGpu::{begin, write_bitmap, write_rgba, commit}
Mission Creator ──RenderEngine::with_terrain_line_of_sight_overlay──▶ TerrainLineOfSightOverlay::{viewshed_upload, viewshed_clear}
every layer ──&mut dyn LaneSink──▶ the renderer's lanes (textured lanes keep a TexLane beside the batch)
```

The renderer holds one of each layer as a field and lends it its lanes for each call. Geometry sent
to the GPU is measured from `map_coordinates::terrain_frames::ANCHOR`, so its `f32` coordinates
stay within 6400 m of zero; a textured lane is one quad over its world rectangle, drawn with the
density pipeline for the forest fill and the plain textured pipeline otherwise. The source README
details each layer.

## Getting started

Run from the repository root:

```bash
cargo test -p world_layers_gpu                                                    # the error messages, the raster rule, the basemap mode, the textured quad
cargo clippy -p world_layers_gpu --target wasm32-unknown-unknown --all-targets -- -D warnings  # the GPU layers
cargo xtask verify crate-anatomy                                                  # lib.rs, prelude, error, README and manifest shape
```

## Configuration

None: no feature and no environment variable. The GPU layers are selected by the `wasm32` target.

## Public surface

- `building_layer::BuildingLayerGpu`, `forest_layer::ForestLayerGpu`,
  `terrain_texture_layer::TerrainTextureLayerGpu`,
  `terrain_line_of_sight_overlay::{TerrainLineOfSightOverlayGpu, TerrainLineOfSightOverlay}` and
  `textured_lane::TexLane` (WebAssembly).
- `basemap_mode::BasemapMode`; `Error`, `LayerCall`, `Result`; `prelude`: the layers, the record,
  the mode and the error.

## Boundaries

- Depends on: `renderer_core` (`LaneSink`, `LayerContext`, the pipeline ids), `gpu_frame` (draw
  batches, instance buffers, the hairline and indexed mesh uploads), `render_primitives` (the
  building and quad instance layouts, the line vertex, the anchor-relative rectangle),
  `map_draw_lanes` (lane roles and ids), `map_coordinates` (the anchor), `bytemuck`, `thiserror`;
  `wgpu` and `web-sys` (`ImageBitmap`) in the WebAssembly build.
- Used by: `map_renderer`, whose render engine holds the layers, keeps the textured lane records
  and forwards the asset sink's uploads; the Mission Creator and the debug building viewer under
  `crates/frontend/workspaces/` through `RenderEngine::with_terrain_line_of_sight_overlay`.
- Rules: map rendering tier 3 (`cargo xtask verify crate-tiers`); no layer names the renderer; a
  refusal's message starts with its call's stable tag (`LayerCall::tag`).

## Related documentation

- [Map streaming](/documentation/crates/streaming/map_streaming.md) — the loaders whose uploads
  reach these layers through the asset sink.
