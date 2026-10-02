# Map coordinates tests

Unit tests of `map_coordinates`, one file per module, each declared by its module through
`#[path]`.

## Contents

```text
crates/geometry/map_coordinates/src/tests/
├── chunk_math.rs      preload margins, clamped rectangles, id order and the Everon viewport chunk set
├── grid_reference.rs  6-, 8- and 10-figure formatting, edge-label agreement, parsing, refusals
└── rounding.rs        `round` against JavaScript's `Math.round`, halves included
```

## Boundaries

- Depends on: the module each file tests (`super::*` or `crate::<module>`).
- Used by: `cargo test -p map_coordinates`.
- Rules: every expected value is written out by hand, never derived from the code under test.
