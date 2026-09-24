# GPU readback checks

Byte-exact offscreen checks of the render engine's pipelines on the backend the browser gave it:
each check draws a known scene into its own texture, copies it back and compares chosen pixels
with the colours the code expects. `readback_rgba` reads one pixel of the engine's live scene the
same way.

## Contents

```text
apps/website/map-engine/src/diagnostics/readback/
├── compute_cull.rs     the GPU icon cull against the CPU oracle over 512 seeded icons
├── doll.rs             the doll renderer's flat-shaded probes, the depth test included
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

Every file compiles only for wasm32 with the `render` feature. Every check is an async
`#[wasm_bindgen]` method on the render engine (the doll check is on `DollEngine`, the doll renderer)
that clones the device handles it needs and resolves a promise to JSON:
`{"backend", "probes": [{px, py, expect, got, pass, label}], "pass"}`, or `cpu`, `gpu` and `pass`
for the compute cull. A pixel check builds an 800×600 `Rgba8Unorm` target, a camera of its own (the
orthographic camera at the world anchor, zoom 0, or the orbit camera for the doll) and the pipeline
under test (from `crate::frame::pipelines`, or the doll renderer's own), draws, and copies the
target into a buffer whose rows are padded to wgpu's copy alignment (`padded_bytes_per_row`).
`map_read_4` then maps the buffer, polling the device every 4 ms for up to 2000 polls, and returns
the four bytes of one pixel, or `readback-map-timeout` or `readback-map-failed`; the doll check runs
its own copy of that loop. The compute-cull check draws nothing: it culls 512 seeded icons against a
fixed rectangle and reads back the GPU's count.

Opaque colours must match byte for byte. The marquee allows ±1 per channel, because a translucent
blend rounds through the GPU's float pipeline. The compute-cull check passes on WebGL2 without
running, since that backend has no compute cull; elsewhere the GPU count must equal the CPU
oracle's and fall strictly between 0 and 512.

`scene.rs` also reads the live scene. `encode_scene_readback` draws the engine's persistent batch
list through `website_graphics_engine::draw::encode::encode` with offscreen pipelines and a camera
bind group of its own, filling local pipeline and bind-group tables instead of the engine's, so the
next real frame never binds an offscreen target; `readback_rgba(x, y)` returns one pixel of that
draw. The file also holds two engine methods outside the checks: `poll`, which drains pending
`map_async` callbacks and which the render loop calls every frame, and `disable_frame_timing`,
which drops the GPU timer.

## Boundaries

- Depends on: `crate::frame` (the engine, `CLEAR_COLOR`, the pipeline constructors, the packet
  tables and types, the text uniform bytes, the compute cull and its CPU oracle),
  `crate::camera` (orthographic and orbit), `crate::doll::renderer` and `crate::doll::scene` (the
  doll check), `crate::world::scene::ANCHOR`, and `website-graphics-engine` (`draw::encode`,
  `layout::QuadInstance`).
- Used by: the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
  `window.__selfChecks.texture` (`texture_self_check`, published in
  `apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs` and called by the editor gate's
  `selfcheck` smoke); `crate::frame::pump` (`poll`, every frame); the Mission Creator's canvas boot
  and the debug benches (`disable_frame_timing`); `crate::diagnostics::bench`
  (`readback_sleep_ms`). No code in the repository calls the other eight checks or
  `readback_rgba`.
- Rules: a check never writes the engine's frame tables, camera uniform or batch list
  (`encode_scene_readback` takes `&self`), so it can run beside the live render loop; expected
  pixels are exact bytes except the marquee's ±1.
