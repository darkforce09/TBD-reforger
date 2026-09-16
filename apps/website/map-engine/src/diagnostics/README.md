# diagnostics

Readback self-checks, frame benchmarks, timing queries, hardware probes, and browser console output.

## Contents

- `bench`
- `mod.rs`
- `platform`
- `probes`
- `readback`
- `timing`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
