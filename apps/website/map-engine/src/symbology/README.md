# symbology

Bespoke role and vehicle glyphs, side tinting, atlas packing, instance updates, labels, and squad links.

## Contents

- `atlas`
- `instances`
- `labels`
- `links`
- `mod.rs`
- `roles`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.

MGRS, range rings, compass rose, and MSR designators have no source implementation.
