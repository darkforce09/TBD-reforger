# tests

Graphics modules and native regression suites for rendering contracts and camera parity.

## Contents

- `feature_gate_tripwire.rs` — `map_engine_tests_require_all_features`: fails unless the test build has every feature (`world`, `streaming`, `render`).

## Boundaries

Tests keep the original assertions and fixtures; run the crate suite with `--all-features`.
