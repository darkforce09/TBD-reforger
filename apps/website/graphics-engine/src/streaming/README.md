# streaming

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `bridge`
- `buffers`
- `host`
- `loaders`
- `memory`
- `mod.rs`
- `scheduler`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
