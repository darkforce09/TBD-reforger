# World layers source

The source of `world_layers_gpu`: the typed GPU layers of the streamed world, the textured lane
record three of them share, the basemap mode that record reports, the textured quad and raster
rules, the crate's error type and the crate root that declares them.

## Contents

```text
crates/map_rendering/world_layers_gpu/src/
├── basemap_mode.rs                    `BasemapMode`: how a textured lane's texture is laid out, as the statistics report spells it
├── building_layer.rs                  `BuildingLayerGpu`: building fills, outlines and fence strips uploaded as lanes, and their counters (WebAssembly)
├── error.rs                           `Error`, `LayerCall` and `Result`: why a layer refused a raster or texture write
├── forest_layer.rs                    `ForestLayerGpu`: the forest density texture lane, its fill and outline settings, the forest counters (WebAssembly)
├── lib.rs                             the crate root: module header, `mod` lines and the error re-export
├── prelude.rs                         the typed layers, the textured lane record, the basemap mode and the error
├── raster_layout.rs                   the single-bitmap raster rule the forest and viewshed uploads check first
├── terrain_line_of_sight_overlay.rs   `TerrainLineOfSightOverlayGpu` and its at-work view: `viewshed_upload` and `viewshed_clear` (WebAssembly)
├── terrain_texture_layer.rs           `TerrainTextureLayerGpu`: the basemap and hillshade texture lanes, begun, written and committed (WebAssembly)
├── tests/                             the native tests of the error messages, the raster rule, the basemap mode and the textured quad
├── textured_lane.rs                   `TexLane`, the texture record of a textured lane, and the one-quad textured lane upsert (WebAssembly)
└── textured_quad.rs                   the anchor-relative rectangle of a textured quad and the pipeline its lane draws with
```

## How it works

**Buildings.** The draw buffers' rebuild (`chunk_draw_buffers::footprint`) composes the building
fill and outline buffers; `BuildingLayerGpu` turns them into the `WorldBuildings` oriented-quad
lane (ten `f32` a building), the `WorldBuildingsOutline` hairline lane (six `f32` a segment end)
and the `WorldFences` indexed polygon lane (six `f32` a vertex), positions made relative to the
scene anchor. A payload whose length is not a whole number of rows removes its lane; an empty
footprint or fence payload removes its lane only when the lane is hidden.

**Forest.** `map_asset_loading`'s forest mass host uploads the island's density grid once as a
texture (`upload_density`, the `ForestFill` lane, density pipeline); after that each frame changes
only the fill's opacity and whether the fill and the outline show (`set_params`, density mode
only). The layer keeps the forest counters the statistics report reads, and the renderer's vector
lane uploads record the vector forest counts into it.

**Terrain textures.** A texture is one of two roles: 0, the basemap (the satellite image or the
cartographic map), and 1, the hillshade. The satellite loads and the map host call, through the
renderer's asset sink:

```text
begin          allocate an RGBA8 texture with its mips as the role's pending texture
               (a role other than 0 or 1, or a zero size or mip count, is refused)
write_bitmap   copy a decoded browser bitmap into one level at (x, y)
write_rgba     copy RGBA bytes into one level at (x, y); the length must be w * h * 4
commit         draw the pending texture as one quad over its world rectangle, tinted to the
               opacity, in place of the role's `Satellite` or `Hillshade` lane
```

A write or commit before `begin` is refused, and a texture with mips counts a third more bytes
than its base level.

**Viewshed.** The Mission Creator's line-of-sight tool packs a finished raster into RGBA rows and
calls `RenderEngine::with_terrain_line_of_sight_overlay`, which lends the renderer's lanes to the
`TerrainLineOfSightOverlayGpu` it holds as a `TerrainLineOfSightOverlay`; `viewshed_upload` shows
the rows as the `Viewshed` lane over the raster's world rectangle and `viewshed_clear` removes it.

**Shared rules.** The forest and viewshed rasters pass `raster_layout::check_raster` first (no
zero dimension, rows at least four bytes a texel and 256-byte aligned, `bytes_per_row × height`
bytes). `textured_quad::world_rect_rel` turns a world rectangle into anchor-relative metres, and
`textured_lane`'s upsert writes the quad as a textured lane with the pipeline
`textured_quad::textured_pipeline_for` names, keeping its `TexLane` record beside the batch. A
record's fields are written only here; the renderer reads them through the accessors.

## Public surface

- `building_layer::BuildingLayerGpu`: `new`, `uploads`, `chunks_drawn`, `upload_buildings`,
  `upload_outlines`, `upload_fence_strips`.
- `forest_layer::ForestLayerGpu`: `new`, `polygons`, `outline_segments`, `density_width`,
  `density_height`, `bins_loaded`, `mode`, `record_fill_polygons`, `record_outline_segments`,
  `upload_density`, `set_params`, `set_outline_stored`.
- `terrain_texture_layer::TerrainTextureLayerGpu`: `new`, `begin`, `write_bitmap`, `write_rgba`,
  `commit`.
- `terrain_line_of_sight_overlay`: `TerrainLineOfSightOverlayGpu` (`new`, `at_work`) and
  `TerrainLineOfSightOverlay` (`viewshed_upload`, `viewshed_clear`).
- `textured_lane::TexLane`: `texture`, `bind_group`, `mode`, `tiles`, `bytes`.
- `basemap_mode::BasemapMode`: `as_str`, `from_u32`; `Error`, `LayerCall` (`tag`), `Result`.

## Boundaries

- Depends on: `renderer_core`, `gpu_frame`, `render_primitives`, `map_draw_lanes`,
  `map_coordinates`, `bytemuck`, `thiserror`; `wgpu` and `web-sys` in the WebAssembly build.
- Used by: `map_renderer`'s render engine (`crates/map_rendering/map_renderer/src/`: boot,
  engine, typed layers, lane sinks, encode, lifecycle, asset sink), which the diagnostics bench
  drives.
- Rules: no file here names the renderer; `basemap_mode.rs`, `error.rs` and `prelude.rs` compile on
  every target, `raster_layout.rs` and `textured_quad.rs` for `wasm32` and the native tests, every
  other module for `wasm32` only.
