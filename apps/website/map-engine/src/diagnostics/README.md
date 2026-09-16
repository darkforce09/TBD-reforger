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

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
