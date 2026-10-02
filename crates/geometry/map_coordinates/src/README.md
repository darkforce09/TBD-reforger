# Map coordinates source

The source of `map_coordinates`: the terrain facts, the chunk grid, the rounding rule and the grid
reference, the error they share, and the crate root that declares them.

## Contents

```text
crates/geometry/map_coordinates/src/
├── chunk_math.rs      viewport box to chunk ids: preload margin, clamped chunk rects, row-major id order
├── error.rs           `Error` and `Result`: a refused grid reference, gathered for `?`
├── grid_reference.rs  the 1 km grid spacing, and grid references formatted and parsed at 6, 8 and 10 figures
├── lib.rs             the crate root: module header, `mod` lines and re-exports
├── prelude.rs         the names most callers import
├── rounding.rs        `round`, JavaScript's `Math.round`: halves round toward +∞
├── terrain_frames.rs  Everon's anchor, bounds and opening view; Arland's centre
└── tests/             unit tests, one file per module
```

## How it works

Each module is pure: constants and functions over world metres, x east and y north.
`chunk_math::chunk_ids_for_viewport` composes the other chunk functions (margin, expanded box,
clamped rectangle, optional ring, ids). `grid_reference::parse_grid` returns a
`GridParseError` naming why it refused the text; `error::Error` wraps it so a caller can gather it
with other coordinate failures.

## Boundaries

- Depends on: `thiserror`.
- Used by: `camera_math` and the map engine (see the crate README).
- Rules: tests live in `tests/`, one file per module, declared through `#[path]`.
