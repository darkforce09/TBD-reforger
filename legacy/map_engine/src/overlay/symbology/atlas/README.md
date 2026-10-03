# Symbology glyph atlas

The browser upload of a finished [slot](/documentation/glossary/n_to_z.md#slot) and symbology
glyph atlas to the GPU. The CPU rasteriser that builds the atlas is
`unit_symbology::symbol_atlas`.

## Contents

```text
legacy/map_engine/src/overlay/symbology/atlas/
├── gpu.rs     `RenderEngine::upload_glyph_atlas`; `wasm32` with `render` only
└── mod.rs     the module tree: `gpu`
```

## How it works

`unit_symbology::symbol_atlas` builds the two-cell slot atlas and appends the unit, vehicle and
comment cells (`extend_atlas_with_unit_glyphs`). On `wasm32` with the `render` feature,
`upload_glyph_atlas` checks the UV count against the graphics engine's `ATLAS_GLYPH_COUNT`, packs
the icon uniforms, builds the atlas with `crate::frame::create_glyph_atlas` and destroys the
texture and buffer it replaces. The instance tint multiplies the white cells, so side colour and
selection colour come from the instance, not the atlas.

## Boundaries

- Depends on: `crate::frame` (`RenderEngine`,
  `GlyphAtlasGpu`, `create_glyph_atlas`) and
  `render_primitives::draw::instances::ATLAS_GLYPH_COUNT` for the upload; `wasm_bindgen`.
- Used by: `crate::frame` (the engine holds and encodes the uploaded `GlyphAtlasGpu`); the
  slot bridge in `crate::overlay::symbology::instances` widens the atlas with
  `unit_symbology::symbol_atlas` before its own upload.
- Rules: the cell layout and its refusals are pinned with the crates
  (`extend_atlas_refuses_a_strip_it_cannot_read` in
  `crates/map_overlay/overlay_instances/src/tests/slot_instances/cases_1.rs`).
