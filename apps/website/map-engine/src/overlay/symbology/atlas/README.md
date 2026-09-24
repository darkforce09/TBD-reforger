# Symbology glyph atlas

The CPU rasteriser of the [slot](/documentation_v2/glossary.md#slot) and symbology glyph atlas, and
the browser upload of a finished
atlas to the GPU. The cells are the map's own vocabulary (unit roles, vehicle silhouettes, the
comment bubble), which is why they are drawn here rather than in the graphics engine.

## Contents

```text
apps/website/map-engine/src/overlay/symbology/atlas/
├── gpu.rs     `RenderEngine::upload_glyph_atlas`; `wasm32` with `render` only
├── mod.rs     the module tree
└── raster.rs  the slot atlas, the symbology cell layout, `extend_atlas_with_unit_glyphs`
```

## How it works

`build_slot_atlas` draws the base strip: two 64 px cells (`SLOT_ATLAS_W` 128 by `SLOT_ATLAS_H` 64),
a ring and a solid disc, with the UV table `SLOT_ATLAS_UV`. `extend_atlas_with_unit_glyphs` takes
any horizontal strip of `ATLAS_CELL_PX` (64 px) cells, refuses one of another shape, copies it
verbatim and appends `SYMBOLOGY_CELL_COUNT` (15) white-on-alpha cells, returning the widened pixels,
a UV quad per cell and the count of base cells (`WidenedSlotAtlas`):

| Offset after the base cells | Cells |
|---|---|
| `UNIT_CELL_BASE` (0) to 4 | the five `UnitRoleClass` glyphs: disc, role knockout, facing point |
| `UNIT_SELECTED_CELL_BASE` (5) to 9 | the same five with the selection ring |
| `VEHICLE_CELL_BASE` (10) to 12 | the three `VehicleKind` silhouettes |
| `COMMENT_CELL` (13), `COMMENT_SELECTED_CELL` (14) | the comment bubble, then with the ring |

The instance tint multiplies the white cells, so side colour and selection colour come from the
instance, not the atlas. On `wasm32` with the `render` feature, `upload_glyph_atlas` checks the UV
count against the graphics engine's `ATLAS_GLYPH_COUNT`, packs the icon uniforms, builds the atlas
with `crate::frame::create_glyph_atlas` and destroys the texture and buffer it replaces.

## Boundaries

- Depends on: `crate::frame` (`RenderEngine`, `GlyphAtlasGpu`, `create_glyph_atlas`) and
  `website_graphics_engine::layout::ATLAS_GLYPH_COUNT` for the upload; `wasm_bindgen`.
- Used by: `crate::overlay::symbology::instances` (the bridge widens the atlas, the symbol packer
  names the cell offsets, and `slots` re-exports the layout), `crate::frame` (the engine holds the
  uploaded `GlyphAtlasGpu`), and the symbology tests in
  `apps/website/map-engine/src/overlay/symbology/tests/`.
- Rules: a strip that is not a row of 64 px cells is refused, never mangled
  (`extend_atlas_refuses_a_strip_it_cannot_read`), and the base cells are copied verbatim
  (`extend_atlas_copies_the_base_verbatim_and_bases_after_it`), both in
  `apps/website/map-engine/src/overlay/symbology/instances/slots/tests/cases_1.rs`; the symbology
  cells are pairwise distinct shapes (`symbology_cells_are_pairwise_distinct_shapes`).
