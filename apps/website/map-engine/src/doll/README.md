# Doll preview

The 3D character preview of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
[arsenal](/documentation_v2/glossary/a_to_f.md#arsenal): a schematic soldier whose equipment regions show
what a loadout fills, turn with a drag, highlight under the pointer and answer clicks. The module
holds the scene, the picking and the renderer; the page holds only the DOM around the canvas.

## Contents

```text
apps/website/map-engine/src/doll/
├── interaction/  ray picking of a region under a pixel, and each region's callout anchor on screen
├── mod.rs        the module tree
├── renderer/     `DollEngine`, the wgpu renderer of the preview, and its instance stream packing
└── scene/        the 14 regions, the soldier's parts, the state colours and the unit meshes
```

## How it works

```text
arsenal page (apps/website/frontend/src/v2/apps/editor/arsenal/doll.rs)
   │ create, resize, rotate, set_states, set_hover, pick_region, anchor_px, render
   ▼
renderer/     DollEngine: own device and surface, damage-driven frames
   │ packs parts and colours           │ picks and anchors with the engine's yaw and size
   ▼                                    ▼
scene/        regions, parts, colours, meshes  ◄──  interaction/  pick, anchor_px
   │
   ▼
crate::camera::orbit   view_proj_gl for picking, view_proj_wgpu for drawing
```

The page pushes one state byte per region (empty, equipped, active) and the hovered region; the
renderer repacks the instance colours and draws only when something changed. A pointer position
goes back through `pick_region` to `interaction/`, which casts a ray with the same camera matrix
the renderer draws with, so the region it returns is the one drawn under the pointer. Each frame
the page asks `anchor_px` where the active region's callout belongs.

`scene/` and `interaction/` are plain Rust and compile with the `render` feature on any target,
which is how the crate's native tests exercise them; `renderer/` apart from its packing needs
wasm32 as well.

## Public surface

- `renderer::lifecycle_1::DollEngine`: the preview's JavaScript-facing engine, for the arsenal page
  in `apps/website/frontend/src/v2/apps/editor/arsenal/doll.rs`.
- `scene::instances` (regions, states, parts, colours) and the renderer's pipeline, pass and
  packing helpers, for the readback check in `crate::diagnostics::readback::doll`.

## Boundaries

- Depends on: `crate::camera::math::glmat4` and `crate::camera::orbit::projection`;
  `crate::frame::boot::instance_descriptor`; the shader
  `apps/website/map-engine/src/shaders/doll.wgsl`; on wasm32, `wgpu`, `wasm-bindgen` and
  `web-sys`; and `bytemuck`.
- Used by: `crate::diagnostics::readback::doll`, the byte-exact offscreen self-check; the arsenal
  page in `apps/website/frontend/src/v2/apps/editor/arsenal/doll.rs`, the only caller outside the
  crate.
- Rules:
  - the module compiles only with the `render` feature
    (`apps/website/map-engine/src/lib.rs`), and its browser code (`wasm_bindgen`, `web_sys`) only
    for wasm32;
  - it imports no UI crate and reads no editor state: the page passes everything in through
    `DollEngine`'s calls;
  - nothing under `crate::data` may name it (rules 4 and 7 of `cargo xtask verify engine-layers`).

## Related documentation

- [Arsenal](/apps/website/frontend/src/v2/apps/editor/arsenal/README.md) — the workspace that
  mounts the preview and owns the loadout it shows.
- [Orbit camera](/apps/website/map-engine/src/camera/orbit/README.md) — the fixed-orbit camera the
  preview draws and picks with.
