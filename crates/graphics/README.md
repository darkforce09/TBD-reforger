# Graphics crates

The map-agnostic renderer crates: building blocks that know nothing about what is being drawn,
only geometry, byte layouts, glyphs and shader text.

## Contents

```text
crates/graphics/
└── render_primitives/  `render_primitives`: GPU-free layouts, geometry, glyph packing and the WGSL source
```

## Boundaries

- Depends on: external crates and lower-tier workspace crates only.
- Used by: the graphics engine (`legacy/graphics_engine/`).
- Rules: a graphics crate declares `category = "crates/graphics"` (`cargo xtask verify
  crate-tiers`), and no name or document in it names a thing in the world being drawn.
