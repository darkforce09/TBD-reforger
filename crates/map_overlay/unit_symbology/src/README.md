# Unit symbology source

The source of `unit_symbology`: the side and class tables, the symbol atlas raster, the briefing
markers, the squad links, the slot id, and the crate root.

## Contents

```text
crates/map_overlay/unit_symbology/src/
├── classification.rs  side tints, `UnitRoleClass`, `VehicleKind`, the role and alias tables
├── lib.rs             the crate root: module header, `mod` lines
├── markers.rs         `MarkerGlyph`: icon aliases to eleven glyphs, their atlas, captions
├── prelude.rs         the names most callers import
├── slot_ids.rs        `SlotId`
├── squad_links.rs     the squad leader-to-member hairlines, at rest and mid-drag
├── symbol_atlas.rs    the slot atlas and the unit, vehicle and comment cells appended to it
└── tests/             unit tests for the markers, the squad links and the slot id wire form
```

## How it works

`symbol_atlas.rs` and `markers.rs` rasterise with 1 px analytic edge coverage into owned RGBA
buffers, white on alpha, so an instance tint colours them. `squad_links.rs` reads positions by
slot id and tints through `classification.rs`.

## Boundaries

- Depends on: `map_draw_lanes::zoom_gates`, `render_primitives::text`, `newtype_ids`.
- Used by: the crate's callers through its modules and `prelude`.
- Rules: the cases in `tests/` pin the marker vocabulary and atlas, glyph ink placement, caption
  packing and the squad link rules.
