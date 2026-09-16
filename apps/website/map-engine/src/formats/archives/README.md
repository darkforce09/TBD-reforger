# formats/archives

Validated binary containers, POD layouts, rkyv archives, and density codecs shared by producers and consumers.

## Contents

- `blueprints.rs`
- `codec.rs`
- `forest.rs`
- `labels.rs`
- `mod.rs`
- `models`
- `prefabs.rs`
- `roads.rs`
- `satellite.rs`
- `tests`
- `version.rs`
- `water.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
