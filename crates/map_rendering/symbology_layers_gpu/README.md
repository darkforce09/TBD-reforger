# Symbology layers on the GPU

The `symbology_layers_gpu` crate: the typed GPU layers of the map symbology. `SlotSymbologyGpu`
owns the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
[slot](/documentation/glossary/n_to_z.md#slot) atlas, the slot bridge's columns, selection and
drag, and the pooled buffers the slot, drag, cluster, preview, vehicle and comment lanes draw
from; `SlotSymbology` is that state at work with the renderer parts a bind writes through.
`GlyphAtlasGpu` owns the glyph atlas the streamed world's icon lanes sample, `IconCullGpu` the
compute frustum cull of the icon lanes, `upload_world_icon_lane` uploads the world's tree, prop
and badge lanes, and `lane_preferences` toggles the world layers, re-tints the texture lanes and
builds the 1 km grid. Every lane write goes through `renderer_core`'s `LaneSink`.

## Contents

```text
crates/map_rendering/symbology_layers_gpu/
├── Cargo.toml  the package: `renderer_core`, `gpu_frame`, `gpu_device`, the map overlay and geometry crates; `wgpu` on wasm32; layout tier 4, wasm32
└── src/        the typed layers, the lane preferences, the icon uniform layout, the error type and their tests
```

## How it works

```text
Mission Creator ──RenderEngine::with_symbology──▶ SlotSymbologyGpu::at_work ─▶ SlotSymbology
  binds, selection, drag, clusters ─▶ pooled lanes ──▶ LaneSink (the renderer's lanes)
                                                   └─▶ IconCullGpu (when the compute cull is in use)
world loader ─▶ renderer's asset sink ─▶ GlyphAtlasGpu::upload, upload_world_icon_lane,
                                         lane_preferences::{set_world_layer_visible, set_lane_opacity, set_grid}
renderer: frame packet ◀── atlas bind groups (SlotSymbologyGpu, GlyphAtlasGpu); cull pass ◀── IconCullGpu
```

The renderer holds one `SlotSymbologyGpu`, one `GlyphAtlasGpu` and one `IconCullGpu` as fields.
It lends the slot symbology out with its lane sink, compute cull, camera, shared text atlas and
uniform byte counter for each call, runs its camera hook through `SlotSymbology::camera_changed`,
and binds the atlases' bind groups at the frame packet's sprite atlas slots. The source README
details each module.

## Getting started

Run from the repository root:

```bash
cargo test -p symbology_layers_gpu                                                    # the icon uniform layout and the error messages
cargo clippy -p symbology_layers_gpu --target wasm32-unknown-unknown --all-targets -- -D warnings  # the GPU layers
cargo xtask verify crate-anatomy                                                      # lib.rs, prelude, error, README and manifest shape
```

## Configuration

None: no feature and no environment variable. The GPU layers are selected by the `wasm32` target.

## Public surface

- `slot_symbology::{SlotSymbologyGpu, SlotSymbology, TextAtlasSupply, is_pooled_icon_lane}`
  (WebAssembly).
- `glyph_atlas_gpu::GlyphAtlasGpu`, `icon_cull_gpu::IconCullGpu`,
  `world_icon_lanes::upload_world_icon_lane` and `lane_preferences` (WebAssembly).
- `icon_uniforms`: the `ICON_*` layout constants, `pack_icon_uniforms`,
  `convert_icon_world_to_anchor`, `sprite_atlas_for`.
- `Error`, `Result`; `prelude`: the layers and the error.

## Boundaries

- Depends on: `renderer_core` (`LaneSink`, `LayerContext`, packet binding ids), `gpu_frame`
  (draw batches, instance buffers, glyph runs, line uploads, the compute cull, the glyph atlas
  builder), `gpu_device` (the lane pool), `render_primitives`, `overlay_instances`,
  `unit_symbology`, `map_draw_lanes`, `spatial_indexes`, `map_coordinates`, `camera_math`,
  `bytemuck`, `thiserror`; `wgpu` in the WebAssembly build.
- Used by: `map_renderer`, whose render engine holds the layers, and `map_render_diagnostics`;
  the Mission Creator and the mortar map picker under `apps/frontend/src/` through
  `RenderEngine::with_symbology`.
- Rules: map rendering tier 4 (`cargo xtask verify crate-tiers`); no layer names the renderer;
  the bind bodies are pinned by `map_renderer`'s lane-bind source pins
  (`crates/map_rendering/map_renderer/src/tests/lane_bind_source_pins/`).

## Related documentation

- [Map symbology](/documentation/design_system/map_symbology.md) — the unit, vehicle and marker
  symbols and side tints.
- [Mission Creator feature inventory: performance at scale](/documentation/apps/frontend/workspaces/editor/feature_inventory/performance_at_scale.md)
  — the selection patches, drag overlay and clusters at scale.
