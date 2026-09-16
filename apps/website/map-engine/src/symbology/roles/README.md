# symbology/roles

Bespoke role and vehicle glyphs, side tinting, atlas packing, instance updates, labels, and squad links.

## Contents

- `classify.rs`
- `mod.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.

Symbols are bespoke: five unit roles, three vehicle kinds, and side tint. MIL-STD-2525/APP-6 affiliation frames, echelon modifiers, and a Civilian side are absent.
