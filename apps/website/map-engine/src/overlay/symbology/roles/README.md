# symbology/roles

Bespoke role and vehicle glyphs, side tinting, atlas packing, instance updates, labels, and squad links.

## Contents

- `classify.rs`
- `mod.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)

Symbols are bespoke: five unit roles, three vehicle kinds, and side tint. MIL-STD-2525/APP-6 affiliation frames, echelon modifiers, and a Civilian side are absent.
