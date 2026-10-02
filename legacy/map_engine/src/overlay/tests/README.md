# overlay/tests

Unit tests of the map overlay: the ordered draw-lane contract, level-of-detail thresholds and the
fire-mission marks, each declared by its module through `#[path = "tests/<file>.rs"]`.

## Contents

- `draw_order.rs`
- `fire_mission_marks_tests.rs`
- `lod_tests.rs`
- `tests`

## Boundaries

Tests keep the original assertions and fixtures; run the crate suite with `--all-features`.
