# Label layout source

The source of `label_layout`: the label declutter, the town importance rules, the world glyph
sizing, the label glyph packing, the ids, and the crate root.

## Contents

```text
crates/map_overlay/label_layout/src/
├── declutter.rs     `LabelSpec` and the distance and importance declutter
├── glyph_math.rs    world object glyph sizes, hex tints, rotation handedness, building icon keys
├── importance.rs    `LocationLabel` and the town draw, declutter and fade rules
├── label_ids.rs     `LabelId` and `LocationId`
├── lib.rs           the crate root: module header, `mod` lines
├── prelude.rs       the names most callers import
├── tests/           unit tests for the declutter, importance, glyph sizing, packing and ids
└── text_packing.rs  decluttered labels into monospaced glyph instances
```

## How it works

`text_packing.rs` runs `declutter.rs` and hands the survivors to the renderer's glyph layout as
`GlyphSpec`s, carrying `LabelId` as the glyph spec id. `importance.rs` works on the location rows
directly, so a town's draw decision needs no label spec. `glyph_math.rs` holds no packing: the
bytes are written by `render_primitives::text::pack`, the one home of glyph packing.

## Boundaries

- Depends on: `render_primitives::text`, `newtype_ids`, `serde`.
- Used by: the crate's callers through its modules and `prelude`.
- Rules: the cases in `tests/` pin the declutter invariant, the town rules, the glyph sizing, the
  packing against the baked atlas and the id serialisation.
