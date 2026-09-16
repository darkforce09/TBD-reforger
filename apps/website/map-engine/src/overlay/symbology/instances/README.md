# symbology/instances

Bespoke role and vehicle glyphs, side tinting, atlas packing, instance updates, labels, and squad links.

## Contents

- `bridge_1.rs`
- `bridge_2.rs`
- `bridge_3.rs`
- `drag.rs`
- `lanes.rs`
- `mod.rs`
- `packing.rs`
- `patches.rs`
- `slots`
- `symbols.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
