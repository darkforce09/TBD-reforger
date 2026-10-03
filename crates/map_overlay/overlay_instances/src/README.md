# Overlay instances source

The source of `overlay_instances`: the symbol packers, the drag previews, the row patches, the
fire-mission marks, and the crate root.

## Contents

```text
crates/map_overlay/overlay_instances/src/
├── drag.rs                `DragGpuPhase`, the drag transition rule, drag overlay and preview packers
├── fire_mission_marks.rs  gun, target and burst glyphs, gun→target lines, dispersion ellipses
├── lib.rs                 the crate root: module header, `mod` lines
├── patches.rs             12-byte row patches for selection and hiding, the selection-only pack
├── prelude.rs             the names most callers import
├── symbols.rs             instance sizes and colours, the cluster gate, slot, vehicle, comment packers
└── tests/                 unit tests for the slot instances and the fire-mission marks
```

## How it works

`drag.rs` and `patches.rs` pack through `symbols.rs`, so every instance shares one layout and one
glyph choice. `fire_mission_marks.rs` writes `LineVertex` geometry relative to the caller's
anchor and skips any mark with a non-finite input.

## Boundaries

- Depends on: `unit_symbology`, `map_draw_lanes`, `render_primitives`.
- Used by: the crate's callers through its modules and `prelude`.
- Rules: the cases in `tests/slot_instances/` and `tests/fire_mission_marks_tests.rs` pin the
  instance bytes, tints, patches, drag rules and fire-mission geometry.
