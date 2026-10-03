# Map symbology

The map engine's symbology as the browser draws it: the upload of the symbol glyph atlas, the
icon instance [slot](/documentation/glossary/n_to_z.md#slot) GPU bridge. The CPU symbology lives
in the map overlay crates (`label_layout`, `unit_symbology`, `overlay_instances`), which every
caller imports directly. It holds what the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) and the streamed world draw,
so the graphics engine only ever sees cells, instances and uniforms.

## Contents

```text
legacy/map_engine/src/overlay/symbology/
├── atlas/      the glyph atlas upload
├── instances/  the slot GPU bridge, the world icon lanes, and the `symbols` re-export
└── mod.rs      the module tree: `atlas` and `instances`
```

## How it works

```text
document rows / streamed world
  │ unit_symbology::classification   role, alias, side ──► glyph class, tint
  │ label_layout                     which labels draw; glyph size and yaw
  │ unit_symbology::squad_links      squad tethers
  │ unit_symbology::markers          marker alias ──► cell; caption glyphs
  ▼
overlay_instances, label_layout::text_packing   20-byte icon and text instances
  │ unit_symbology::symbol_atlas                 cells the instances index
  ▼
RenderEngine lanes (instances/ bridge, atlas/gpu.rs, frame/)  ──► graphics_engine draws
```

This module holds only the browser half: the atlas upload and the instance lane bridges.
`instances::symbols` re-exports `overlay_instances::symbols`, the path `crate::camera::viewport`
reads `cluster_mode` through. Browser I/O (the atlas upload, the lane binds)
compiles only on `wasm32` with the `render` feature.

## Public surface

- `atlas` and `instances`: see their READMEs.

## Boundaries

- Depends on: `unit_symbology`, `overlay_instances` (`symbols` re-exported), `map_draw_lanes`,
  `spatial_indexes`, `crate::frame` (`RenderEngine` and its lanes),
  `map_coordinates::terrain_frames`, `render_primitives` and `wasm_bindgen`.
- Used by: `crate::frame` (the atlas and slot GPU state), `crate::camera` (the `symbols`
  re-export), `crate::streaming` (world icon lanes); the Mission Creator in
  `apps/frontend/src/workspaces/editor/` through the `RenderEngine` methods.
- Rules: the symbology's own cases live with the crates (`crates/map_overlay/`); the side tints
  are pinned by `cargo xtask verify editor-orbat-coherency`; no name in the graphics engine may
  say symbology, which is why this vocabulary lives with the map (rule 2 of
  `cargo xtask verify engine-layers`).

## Related documentation

- [Map symbology](/documentation/design_system/map_symbology.md) — the unit, vehicle and
  marker symbols and side tints, and how the game draws the same markers.
- [Design tokens](/documentation/design_system/design_tokens.md) — the palette the side tints
  come from.
