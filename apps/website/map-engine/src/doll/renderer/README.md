# Doll renderer

The GPU side of the doll preview in the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
[arsenal](/documentation_v2/glossary/a_to_f.md#arsenal):
`DollEngine`, a second wgpu engine with its own device and canvas surface that draws the mannequin
as instanced cubes and one cylinder with depth testing, and the byte packing of the instance
stream it draws from.

## Contents

```text
apps/website/map-engine/src/doll/renderer/
├── engine/         re-exports `DollEngine` under `renderer::engine`
├── lifecycle_1.rs  `DollEngine`: creation, resize, turn, hover, region states, picks and anchors
├── lifecycle_2.rs  `DollEngine::render`, the damage-driven frame
├── mod.rs          the module tree; every module but `pack` needs wasm32 and `render`
├── pack.rs         `pack_instances`: the 80-byte instance stream and its cube and cylinder counts
├── pass.rs         the render pass (clear colour, depth clear) and the two instanced draws
├── pipeline.rs     the render pipeline and the `Depth32Float` depth target
└── tests/          unit tests for the instance stream's layout, order and colour rewrites
```

## How it works

```text
DollEngine::create(canvas, force_webgl)
   │ WebGPU when the browser has it, else WebGL2 (always WebGL2 when force_webgl)
   │ non-sRGB surface format, Fifo present; shader src/shaders/doll.wgsl
   │ cube mesh and 16-segment cylinder mesh from crate::doll::scene::mesh
   ▼
set_states / set_hover ──> pack_instances ──> instance buffer (queue.write_buffer)
rotate / resize ─────────> yaw, CSS size, depth target
   │ each call sets dirty
   ▼
render()   no-op unless dirty or continuous
   │ uniform = view_proj_wgpu(yaw, CSS size) + a zero params vec4 (80 bytes, lit path)
   ▼
doll_pass: clear to CLEAR_COLOR, depth 1.0 ──> draw_doll: cubes, then the cylinders
```

`pack_instances(states, hover)` walks `crate::doll::scene::instances::instances()` and writes each
part as 80 bytes (`INSTANCE_STRIDE`): its model matrix as 16 `f32` and its colour as 4 `f32`. All
cubes come first and the cylinders after them, so `draw_doll` binds the one buffer twice at
different offsets. A part's colour is `decor_color()` for body decor, otherwise `state_color` of
its region's state, lifted when the region is hovered. The pipeline reads the mesh at 24 bytes a
vertex (position and normal) and the instance as five `Float32x4` attributes, with no culling, no
blending and a `Less` depth test.

`create` fails with a message when the canvas has no backing size (`canvas-zero-size: …`), when no
adapter or device comes back, or when the surface offers only sRGB formats. It sets the CSS size
from the canvas's device pixels, so a host calls `resize(css_w, css_h, dpr)` before its first pick.
`set_states` takes exactly 14 bytes in `REGION_KEYS` order (0 empty, 1 equipped, 2 active) and
fails otherwise; `set_hover` does nothing when the region has not changed. `rotate(dx_px)` turns
the yaw by `-0.012` rad a pixel. `render` skips a frame when the surface times out or is occluded,
reconfigures once on an outdated or lost surface, and fails if the texture still cannot be taken.
`set_continuous_render(true)` draws every frame; `mark_dirty` forces the next one.

## Public surface

- `lifecycle_1::DollEngine`, exported to JavaScript through `wasm_bindgen`: `create`, `backend`
  (`"webgpu"` or `"webgl2"`), `resize`, `rotate`, `set_hover`, `set_states`, `pick_region`,
  `anchor_px`, `mark_dirty`, `set_continuous_render` and `render`, for the arsenal preview.
- `lifecycle_1::UNIFORM_SIZE`, `pack::pack_instances`, `pass::doll_pass`, `pass::draw_doll`,
  `pipeline::create_doll_pipeline` and `pipeline::create_depth`, crate-visible, for the readback
  check in `crate::diagnostics::readback::doll`, which adds `DollEngine::doll_self_check`.

## Boundaries

- Depends on: `crate::doll::scene` (instances, colours, meshes), `crate::doll::interaction`
  (picks and anchors), `crate::camera::orbit::projection::view_proj_wgpu`,
  `crate::frame::boot::instance_descriptor` and the WGSL source
  `apps/website/map-engine/src/shaders/doll.wgsl`; the crates `wgpu`, `wasm-bindgen`, `web-sys`
  and `bytemuck`.
- Used by:
  - `crate::diagnostics::readback::doll`, the byte-exact offscreen self-check;
  - the arsenal preview in `apps/website/frontend/src/v2/apps/editor/arsenal/doll.rs`, which
    creates the engine, forwards pointer moves, drags and clicks, pushes the states and runs the
    `requestAnimationFrame` loop.
- Rules:
  - the renderer names no `website_graphics_engine` module (rules 3a and 3b of
    `cargo xtask verify engine-layers`): it draws with `wgpu` directly and shares only
    `instance_descriptor` with the map's render engine;
  - the instance stream is 80 bytes a part, cubes before cylinders, with the launcher as the one
    cylinder, and a state or hover change rewrites only the affected region's colours
    (`byte_layout_golden`, `cubes_stream_before_cylinders_and_launcher_is_the_cylinder`,
    `state_flip_rewrites_exactly_the_region_colors` and
    `hover_flip_rewrites_exactly_the_hovered_region` in `tests/pack_tests.rs`);
  - `INSTANCE_STRIDE` in `pack.rs` matches the five-attribute instance layout in `pipeline.rs` and
    `UNIFORM_SIZE` matches `DollUniforms` in `doll.wgsl`; no test ties the Rust sizes to the
    shader.

## Related documentation

- [Arsenal](/apps/website/frontend/src/v2/apps/editor/arsenal/README.md) — the workspace that
  mounts the preview.
- [Orbit camera](/apps/website/map-engine/src/camera/orbit/README.md) — the camera behind the
  render uniform.
