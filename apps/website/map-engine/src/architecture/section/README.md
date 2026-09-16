# architecture/section

Building blueprint geometry, compound assembly, local-to-world transforms, section cuts, and structure occlusion.

## Contents

- `cutter.rs`
- `index.rs`
- `mod.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
