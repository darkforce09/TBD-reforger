# GPU readback checks

Byte-exact offscreen checks of the render engine's pipelines and its calibration quads on the
backend the browser gave it: each check draws a known scene into its own texture, copies it back
and compares chosen pixels with the colours the code expects. `readback_rgba` reads one pixel of the engine's live scene the
same way.

## Contents

```text
crates/map_rendering/map_render_diagnostics/src/readback/
├── calibration.rs      `calibration_self_check`: seven pixel probes over the engine's two calibration quads
├── compute_cull.rs     the GPU icon cull against the CPU oracle over 512 seeded icons
├── marquee.rs          the selection marquee's translucent fill and its border
├── mod.rs              the module tree
├── road_centerline.rs  a road strip's centreline colour, with the clear colour beside it
├── scene.rs            the shared readback helpers, and `readback_rgba` over the live batch list
├── sea_band.rs         the sea polygon's band colour at its centre and near its corner
├── text.rs             an upright label glyph: ink, descender, halo and the cells a flip would fill
├── texture.rs          a 2×2 texture drawn north-up through the textured pipeline
├── tree_glyph.rs       a tree glyph sampled from the icon atlas in the forest tint
└── world_building.rs   an oriented building quad's fill, its exterior and its rotation
```

## How it works

Every file compiles only for wasm32. Every check is a function that takes the render engine,
clones the device handles it needs from the engine's diagnostic views
(`map_renderer::diagnostic_accessors`) and resolves a promise to JSON:
`{"backend", "probes": [{px, py, expect, got, pass, label}], "pass"}`, or `cpu`, `gpu` and `pass`
for the compute cull. A pixel check builds an 800×600 `Rgba8Unorm` target, a camera of its own (the
orthographic camera at the world anchor, zoom 0) and the pipeline
under test (from `gpu_frame::pipeline`), draws, and copies the
target into a buffer whose rows are padded to wgpu's copy alignment (`padded_bytes_per_row`).
`map_read_4` then maps the buffer, polling the device every 4 ms for up to 2000 polls, and returns
the four bytes of one pixel, or `readback-map-timeout` or `readback-map-failed`. The compute-cull check draws nothing: it culls 512 seeded icons against a
fixed rectangle and reads back the GPU's count. The calibration check draws the engine's two
calibration quads (its first batch) with a fixed 800×600 camera and probes seven pixels, the clear
colour included; its map errors are `probe-map-timeout` and `probe-map-failed`.

Opaque colours must match byte for byte. The marquee allows ±1 per channel, because a translucent
blend rounds through the GPU's float pipeline. The compute-cull check passes on WebGL2 without
running, since that backend has no compute cull; elsewhere the GPU count must equal the CPU
oracle's and fall strictly between 0 and 512.

`scene.rs` also reads the live scene, through the engine's read-only scene view.
`encode_scene_readback` draws the engine's persistent batch list through
`gpu_frame::draw::encode::encode` with offscreen pipelines and a camera bind group of its
own, filling local pipeline and bind-group tables instead of the engine's, so the next real frame
never binds an offscreen target; `readback_rgba(engine, x, y)` returns one pixel of that draw.

## Boundaries

- Depends on: `map_renderer` (the engine's diagnostic views, `CLEAR_COLOR`, the packet's pipeline
  table), `gpu_frame` (the pipeline constructors, the packet type, `draw::encode`, the compute
  cull), `renderer_core::packet_bindings` (the camera binding id),
  `symbology_layers_gpu::icon_uniforms` (the tree glyph's icon uniforms), `render_primitives` (the
  instance layouts, line vertices, camera uniform, text uniform bytes and atlas, and the compute
  cull's CPU oracle), `camera_math` (the orthographic check cameras) and
  `map_coordinates::terrain_frames::ANCHOR`.
- Used by: the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
  `window.__selfChecks.calibration` (`calibration_self_check`) and
  `window.__selfChecks.texture` (`texture_self_check`), both published in
  `apps/frontend/src/workspaces/editor/bridge/viewport.rs` and called by the editor gate's
  `selfcheck` smoke; `crate::benchmark` (`readback_sleep_ms`). The other seven map checks and
  `readback_rgba` hang on the same `window.__selfChecks` under their own names; no gate calls
  them. The Arsenal paper doll's
  `doll_self_check` is `paper_doll_renderer`'s (`crates/paper_doll/paper_doll_renderer/`).
- Rules: a check never writes the engine's frame tables, camera uniform or batch list
  (`encode_scene_readback` takes `&RenderEngine` and reads read-only views), so it can run beside
  the live render loop; expected pixels are exact bytes except the marquee's ±1.
