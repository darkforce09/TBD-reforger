# Paper doll crates

The 3D character preview of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
[arsenal](/documentation/glossary/a_to_f.md#arsenal): a schematic soldier whose equipment regions show
what a loadout fills, turn with a drag, highlight under the pointer and answer clicks. The scene and
the picking are one crate, the renderer another; the Arsenal page holds only the DOM around the
canvas.

## Contents

```text
crates/paper_doll/
├── paper_doll_renderer/  `paper_doll_renderer`: the wgpu renderer of the soldier, its instance packing and its readback self-check
└── paper_doll_scene/     `paper_doll_scene`: the 14 regions, the soldier's parts, the state colours, the unit meshes and the picking
```

## How it works

```text
Arsenal host (apps/frontend/src/workspaces/editor/arsenal/doll.rs)
   │ create, resize, rotate, set_states, set_hover, pick_region, anchor_px, render, self_check
   ▼
paper_doll_renderer   PaperDollRenderer: GPU context from gpu_device, damage-driven frames
   │ packs parts and colours           │ picks and anchors with the renderer's yaw and size
   ▼                                    ▼
paper_doll_scene      regions, parts, colours, meshes, pick, anchor_px
   │
   ▼
camera_math::orbit   view_proj_gl for picking, view_proj_wgpu for drawing
```

The page pushes one state byte per region (empty, equipped, active) and the hovered region; the
renderer repacks the instance colours and draws only when something changed. A pointer position
goes back through `pick_region` to the scene's picking, which casts a ray with the same camera
matrix the renderer draws with, so the region it returns is the one drawn under the pointer. Each
frame the page asks `anchor_px` where the active region's callout belongs.

`paper_doll_scene` is plain Rust for every target, which is how its native tests exercise it;
`paper_doll_renderer` apart from its instance packing needs wasm32.

## Boundaries

- Depends on: `camera_math` (both crates) and `gpu_device` (the renderer).
- Used by: the Arsenal host in `apps/frontend/src/workspaces/editor/arsenal/doll.rs`, the only
  caller, through `paper_doll_renderer`.
- Rules:
  - paper doll category (`cargo xtask verify crate-tiers`): rendering crates that may depend on
    any engine crate; only the renderer may use `wgpu` and the browser crates;
  - neither crate imports a UI crate or reads editor state: the page passes everything in
    through the renderer's calls;
  - no mission crate may name them: a `crates/mission` crate depends on no rendering crate
    (`cargo xtask verify crate-tiers`).

## Related documentation

- [Arsenal](/apps/frontend/src/workspaces/editor/arsenal/README.md) — the workspace that
  mounts the preview and owns the loadout it shows.
- [Orbit camera](/crates/geometry/camera_math/src/orbit/README.md) — the fixed-orbit camera the
  preview draws and picks with.
