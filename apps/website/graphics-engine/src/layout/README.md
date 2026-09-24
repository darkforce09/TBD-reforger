# Shared binary layouts

One list of every byte layout the graphics engine shares with its callers: the instance structs,
the line vertex, the composition results, the text cell metrics and the sprite bit-packing. The
module holds re-exports only, so the cross-crate binary contract reads as a single file.

## Contents

```text
apps/website/graphics-engine/src/layout/
└── mod.rs  re-exports the instance, vertex, composition, text metric and packing items
```

## Boundaries

- Depends on: `crate::draw::instances` (`QuadInstance`, `BuildingInstance`, `IconInstance`,
  `UNIT_QUAD`, `ATLAS_GLYPH_COUNT`, `CHUNK_CAPACITY`), `crate::draw::geometry` (`LineVertex`,
  `corner_uv`, `pack_offset`), `crate::draw::compose` (`PolyMeshGpu`, `HairlineGpu`,
  `mesh_from_tri`, `retint_fill_alpha`, `u8_rgba_to_f32`), `crate::text::metrics` and
  `crate::text::layout` (everything public), `crate::text::scale::REF_ZOOM` and the whole of
  `crate::text::pack`.
- Used by: `website-map-engine` modules that pack bytes next to their own data, without going
  through its `frame/` module: the building and vegetation buffers and the satellite textures in
  `apps/website/map-engine/src/world/`, the symbology atlas and instance lanes in
  `apps/website/map-engine/src/overlay/symbology/`, the terrain line-of-sight overlay in
  `apps/website/map-engine/src/spatial/los/terrain/overlay.rs`, the boot, engine, lifecycle and
  text-upload files of `apps/website/map-engine/src/frame/`, and the readback probes and frame
  bench in `apps/website/map-engine/src/diagnostics/`.
- Rules: everything listed is plain data, a constant that sizes it or a function that writes it;
  nothing here owns a GPU resource or holds logic. Adding an item widens the cross-crate contract
  and belongs in this file alone. `cargo xtask verify engine-layers` leaves this module open to
  the map engine (rule 3b restricts only `device`, `pipeline`, `shaders`, `r#loop` and `text::gpu`).
