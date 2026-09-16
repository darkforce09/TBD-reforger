# architecture/blueprint

Building blueprint geometry, compound assembly, local-to-world transforms, section cuts, and structure occlusion.

## Contents

- `archive.rs`
- `attribution_1.rs`
- `attribution_2.rs`
- `footprint.rs`
- `geometry.rs`
- `mod.rs`
- `model`
- `structure.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
