# World format crates

The engine category for the files a terrain's map data is stored in: the byte layouts the
developer tools write and the map engine reads, each defined once so the two sides cannot
disagree.

## Contents

```text
crates/world_formats/
└── world_file_formats/  `world_file_formats`: rkyv archives, fixed-header containers, density grids, object rows
```

## Boundaries

- Depends on: `newtype_ids` (foundation) for the identifier types, and external crates (`rkyv`,
  `bytemuck`, `thiserror`).
- Used by: the map engine, behind its `io` feature, and the developer tools.
- Rules: a world formats crate declares `category = "crates/world_formats"` and depends only on
  lower engine categories (`cargo xtask verify crate-tiers`).
