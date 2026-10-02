# tests

Graphics modules and native regression suites for rendering contracts and camera parity.

## Contents

- `feature_gate_tripwire.rs`
- `source_scrub.rs` — reduces Rust source to the text a build compiles, for the guards that read this crate's own source; compiled only with `store`, where its callers live.

## Boundaries

Tests keep the original assertions and fixtures; run the crate suite with `--all-features`.
