# diagnostics/timing

Readback self-checks, frame benchmarks, timing queries, hardware probes, and browser console output.

## Contents

- `gpu.rs`
- `mod.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
