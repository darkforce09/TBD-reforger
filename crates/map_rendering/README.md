# Map rendering crates

The rendering category for the map: the typed GPU layers that own what the map draws on the GPU
and the map renderer that holds them. A typed layer owns its GPU state (atlases, uniform blocks,
pooled buffers, compute passes) and writes its lanes through the renderer contracts of
`crates/graphics/renderer_core`; the map renderer implements those contracts, holds the layers as
fields and drives them every frame. Unlike the graphics crates, a map rendering crate may name
the things in the world being drawn.

## Contents

```text
crates/map_rendering/
├── map_render_diagnostics/  `map_render_diagnostics`: the render engine's readback self-checks, scene readback, frame benchmark and stress pool
├── map_renderer/          `map_renderer`: the render engine: GPU context, pipelines, batch list, lane sinks, upload belts, statistics, asset sink
├── symbology_layers_gpu/  `symbology_layers_gpu`: slot symbology, glyph atlas, icon lane cull, world icon lanes, lane preferences
└── world_layers_gpu/      `world_layers_gpu`: building, forest density, satellite and hillshade texture and terrain line of sight overlay layers
```

## How it works

```text
CPU symbology (crates/map_overlay) ─▶ typed GPU layer ──&mut dyn LaneSink──▶ map renderer's lanes
                                          │                                       │
                                          └── atlas bind groups ◀── frame packet ─┘
```

A typed layer turns the CPU side's packed instances into lane batches and GPU resources; the map
renderer lends it a lane sink and a layer context for each call and reads its bind groups when it
fills the frame packet.
The world layers take the browser loaders' uploads (`crates/streaming`), which the map renderer's
asset sink forwards to them, and the map engine keeps re-export shims at its world and spatial
module paths for them. The render diagnostics measure the map renderer from outside it: each
readback check, the benchmark and the stress pool is a function over the engine's diagnostic
views.

## Boundaries

- Depends on: foundation, contracts, mission, ballistics, engine (`crates/geometry`,
  `crates/map_overlay`, `crates/streaming`, …) and graphics crates, and other rendering crates;
  `wgpu` in the WebAssembly build.
- Used by: the single-page app (`apps/frontend`, WebAssembly build only), which mounts
  `map_renderer`'s `RenderEngine` and registers `map_render_diagnostics`' checks; the typed layers
  are used by `map_renderer`, whose render engine holds them.
- Rules: a map rendering crate declares `category = "crates/map_rendering"` and depends only along
  the category matrix (`cargo xtask verify crate-tiers`); no engine crate depends on one.
