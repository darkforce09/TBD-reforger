# streaming/bridge

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `statistics.rs`
- `host_preferences.rs`
- `mod.rs`
- `preferences.rs`
- `progress.rs`
- `toggles.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
