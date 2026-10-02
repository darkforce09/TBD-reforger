# camera/tests

Unit tests for the grid reference: 6-, 8- and 10-figure formatting, agreement with the map's edge
labels, parsing to the cell centre, and the refusals of malformed references.

## Contents

- `grid_reference_tests.rs`

## Boundaries

Declared by `camera/grid_reference.rs` through `#[path]`; run with
`cargo test -p map_engine --lib camera::grid_reference`.
