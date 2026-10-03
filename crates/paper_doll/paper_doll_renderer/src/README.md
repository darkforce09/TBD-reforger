# Paper doll renderer source

The GPU side of the paper doll in the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
[arsenal](/documentation/glossary/a_to_f.md#arsenal): `PaperDollRenderer`, a wgpu renderer on its
own canvas that draws the mannequin of `paper_doll_scene` as instanced cubes and one cylinder with
depth testing, the byte packing of the instance stream it draws from, and its byte-exact readback
self-check.

## Contents

```text
crates/paper_doll/paper_doll_renderer/src/
├── doll_draw.rs         the doll's GPU meshes, the render pass (clear colour, depth clear) and the two instanced draws
├── error.rs             `Error` and `Result`: the GPU context's errors, the region state count, the probe readback
├── frame_render.rs      `PaperDollRenderer::render`, the damage-driven frame
├── instance_packing.rs  `pack_instances`: the 80-byte instance stream and its cube and cylinder counts
├── lib.rs               the crate root; every module but `error`, `instance_packing` and `prelude` needs wasm32
├── pipeline.rs          `DollProgram` (shader, layouts, uniforms), the render pipeline and the `Depth32Float` target
├── prelude.rs           the names an Arsenal host imports
├── renderer.rs          `PaperDollRenderer`: creation, resize, turn, hover, region states, picks and anchors
├── self_check.rs        `PaperDollRenderer::self_check`: the byte-exact offscreen readback probes
├── shaders/             the doll's WGSL program
└── tests/               unit tests for the instance stream's layout, order and colour rewrites
```

## How it works

```text
PaperDollRenderer::create(canvas, force_webgl)
   │ gpu_device::GpuContext::create(canvas, force_webgl, "doll-engine", no timestamps)
   │ shader shaders/doll.wgsl, pipeline for the surface format, uniform and bind group
   │ cube mesh and 16-segment cylinder mesh from paper_doll_scene::part_meshes
   ▼
set_states / set_hover ──> pack_instances ──> instance buffer (queue.write_buffer)
rotate / resize ─────────> yaw, CSS size, GpuContext::resize, depth target
   │ each call sets dirty
   ▼
render()   no-op unless dirty or continuous
   │ uniform = view_proj_wgpu(yaw, CSS size) + a zero params vec4 (80 bytes, lit path)
   │ GpuContext::acquire: an image, or a skip that leaves the frame dirty
   ▼
doll_pass: clear to CLEAR_COLOR, depth 1.0 ──> draw_doll: cubes, then the cylinders ──> present
```

`pack_instances(states, hover)` walks `paper_doll_scene::soldier_parts::instances()` and writes
each part as 80 bytes (`INSTANCE_STRIDE`): its model matrix as 16 `f32` and its colour as 4 `f32`.
All cubes come first and the cylinders after them, so `draw_doll` binds the one buffer twice at
different offsets. A part's colour is `decor_color()` for body decor, otherwise `state_color` of
its region's state, lifted when the region is hovered. The pipeline reads the mesh at 24 bytes a
vertex (position and normal) and the instance as five `Float32x4` attributes, with no culling, no
blending and a `Less` depth test.

`create` fails with `Error::Gpu` when the canvas has no backing size (`canvas-zero-size: …`),
when no adapter or device comes back, or when the surface offers only sRGB formats. It sets the
CSS size from the canvas's device pixels, so a host calls `resize(css_width, css_height,
device_pixel_ratio)` before its first pick; `resize` clamps each device side to at least one
pixel. `set_states` takes exactly `REGION_COUNT` (14) bytes in `REGION_KEYS` order (0 empty,
1 equipped, 2 active) and fails with `Error::RegionStateCount` otherwise; `set_hover` does
nothing when the region has not changed. `rotate(dx_px)` turns the yaw by `-0.012` rad a pixel.
`render` skips a frame when the surface times out or is occluded, the GPU context reconfigures
once on an outdated or lost surface, and the frame fails if the texture still cannot be taken.
`set_continuous_render(true)` draws every frame; `mark_dirty` forces the next one.

`self_check` draws the doll unlit (`params.x = 1`) into an 800 × 600 `Rgba8Unorm` target with the
helmet active and the plate and rifle equipped, copies it to a readback buffer of 3328-byte rows,
polls the mapping every 4 ms (at most 2000 times) and compares five probe pixels with the exact
state colour bytes: the background, the helmet front, the plate front (which the backpack would
paint without the depth test), the rifle receiver and the boot front. It resolves to
`{"backend","probes":[…],"pass"}`.

## Public surface

- `renderer::PaperDollRenderer` (also at the crate root), for the Arsenal host in the
  frontend: `create`, `backend` (`"webgpu"` or `"webgl2"`), `resize`, `rotate`, `set_hover`,
  `set_states`, `pick_region`, `anchor_px`, `mark_dirty`, `set_continuous_render`, `render` and
  `self_check`.
- `instance_packing`: `INSTANCE_STRIDE`, `InstanceStreams` and `pack_instances`.
- `Error` and `Result` at the crate root; the common names in `prelude`.

## Boundaries

- Depends on: `paper_doll_scene` (parts, colours, meshes, picks and anchors),
  `gpu_device::GpuContext` (the canvas's GPU: create, resize, acquire),
  `camera_math::orbit::projection` (`view_proj_wgpu` for the frame, `view_proj_gl` for the probe
  pixels) and `camera_math::matrix4::transform_vector`; the crates `wgpu`, `web-sys`, `js-sys`,
  `wasm-bindgen-futures`, `bytemuck` and `thiserror`.
- Used by: the Arsenal host in `apps/frontend/src/workspaces/editor/arsenal/doll.rs`, which
  creates the renderer, forwards pointer moves, drags and clicks, pushes the states, runs the
  `requestAnimationFrame` loop and registers `window.__arsenalDoll` (`backend`, `anchor`, `pick`,
  `doll_self_check`).
- Rules:
  - the renderer draws with `wgpu` directly and creates its GPU only through `gpu_device`;
  - the instance stream is 80 bytes a part, cubes before cylinders, with the launcher as the one
    cylinder, and a state or hover change rewrites only the affected region's colours
    (`byte_layout_golden`, `cubes_stream_before_cylinders_and_launcher_is_the_cylinder`,
    `state_flip_rewrites_exactly_the_region_colors` and
    `hover_flip_rewrites_exactly_the_hovered_region` in `tests/instance_packing_tests.rs`);
  - `INSTANCE_STRIDE` in `instance_packing.rs` matches the five-attribute instance layout in
    `pipeline.rs` and `UNIFORM_SIZE` matches `DollUniforms` in `shaders/doll.wgsl`; no test ties
    the Rust sizes to the shader.

## Related documentation

- [Arsenal](/apps/frontend/src/workspaces/editor/arsenal/README.md) — the workspace that
  mounts the paper doll.
- [Orbit camera](/crates/geometry/camera_math/src/orbit/README.md) — the camera behind the
  render uniform.
- [GPU device](/crates/graphics/gpu_device/README.md) — the GPU context the renderer creates its
  device with.
