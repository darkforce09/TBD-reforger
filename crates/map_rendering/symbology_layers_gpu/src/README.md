# Symbology layers source

The source of `symbology_layers_gpu`: the typed GPU layers of the map symbology, the lane
preferences, the icon uniform layout they share, the crate's error type and the crate root that
declares them.

## Contents

```text
crates/map_rendering/symbology_layers_gpu/src/
├── error.rs             `Error` and `Result`: why an atlas upload was refused
├── glyph_atlas_gpu.rs   `GlyphAtlasGpu`: the glyph atlas the world icon lanes sample, and its upload (WebAssembly)
├── icon_cull_gpu.rs     `IconCullGpu`: the compute frustum cull of the icon lanes and its tree icon oracle (WebAssembly)
├── icon_uniforms.rs     the icon uniform layout (`ICON_*`), `pack_icon_uniforms`, the anchor conversion, `sprite_atlas_for`
├── lane_preferences.rs  world layer visibility, texture lane opacity and the 1 km grid, over any `LaneSink` (WebAssembly)
├── lib.rs               the crate root: module header, `mod` lines and the error re-export
├── prelude.rs           the typed layers and the error
├── slot_symbology/      `SlotSymbologyGpu` and its working view `SlotSymbology`: slot atlas, binds, selection, drag, clusters
├── tests/               the native tests of the icon uniform layout and the error messages
└── world_icon_lanes.rs  `upload_world_icon_lane`: the streamed world's tree, prop and badge lanes (WebAssembly)
```

## How it works

Every instance is 20 bytes: world position, size, yaw, glyph index and packed RGBA. The uploads
move the position from world metres to metres from the scene anchor
(`convert_icon_world_to_anchor`) before the bytes reach the GPU, and `sprite_atlas_for` names the
atlas each sprite lane samples: the movable-sprite atlas for the slot, cluster, preview, vehicle,
marker and comment lanes (the drag lane with its drag offset), the glyph atlas for every other
sprite lane. The icon uniform block is the UV table of `ATLAS_GLYPH_COUNT` cells followed by the
drag offset and the pixels-to-metres scale; both atlases pack it with `pack_icon_uniforms`.

`GlyphAtlasGpu::upload` refuses a UV table longer than the block, builds the atlas with
`gpu_frame::frame::create_glyph_atlas` on the layer context's device and destroys the atlas it
replaces. `upload_world_icon_lane` uploads the world's tree, prop and badge lanes as glyph-atlas
sprite batches, or hands them to `IconCullGpu` when the compute cull is in use. The lane
preferences map a world layer *name* to its lanes (`roads` is two lanes, casing and surface),
write a texture lane's tint at byte offset 16 of its live instance buffer, and build the 1 km
grid with `render_primitives::draw::grid`.

## Public surface

- `slot_symbology`: see its README.
- `glyph_atlas_gpu::GlyphAtlasGpu`: `new`, `upload`, `is_uploaded`, `bind_group`, `bytes`.
- `icon_cull_gpu::IconCullGpu`: `new`, `enabled`, `has_any_source`, `upload_lane`, `clear_lane`,
  `encode_cull`, `kick_readback`, `lane_draw`, `set_debug_hud`, `last_cpu_count`, `gpu_count`,
  `lane_gpu_count`, `gpu_sampled`, `set_tree_icons`, `clear_tree_icons`,
  `count_tree_icons_in_frustum`.
- `icon_uniforms`: `ICON_UV_BYTES`, `ICON_UNIFORM_BYTES`, `ICON_DRAG_OFF`, `ICON_PXM_OFF`,
  `pack_icon_uniforms`, `convert_icon_world_to_anchor`, `sprite_atlas_for`.
- `lane_preferences`: `set_grid`, `set_world_layer_visible`, `set_lane_opacity`, `grid_lines`.
- `world_icon_lanes::upload_world_icon_lane`; `Error`, `Result`.

## Boundaries

- Depends on: `renderer_core`, `gpu_frame`, `gpu_device`, `render_primitives`,
  `overlay_instances`, `unit_symbology`, `map_draw_lanes`, `spatial_indexes`, `map_coordinates`,
  `camera_math`, `bytemuck`, `thiserror`; `wgpu` in the WebAssembly build.
- Used by: `map_renderer`'s render engine (`crates/map_rendering/map_renderer/src/`: boot,
  encode, cull, statistics, typed layers, asset sink, text upload) and `map_render_diagnostics`
  (benchmark, readback).
- Rules: no file here names the renderer; `icon_uniforms.rs`, `error.rs` and `prelude.rs`
  compile on every target, every other module for `wasm32` only.
