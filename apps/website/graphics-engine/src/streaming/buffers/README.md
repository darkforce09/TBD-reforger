# streaming/buffers

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `glyphs.rs`
- `mod.rs`
- `packer.rs`
- `revision.rs`
- `strips.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
