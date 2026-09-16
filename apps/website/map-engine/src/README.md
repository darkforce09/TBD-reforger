# Graphics engine modules

Graphics modules, the mission data domain (`data`, folded in from `website-mission-core` at
T-0xx Phase 2A), and native regression suites for rendering contracts and camera parity.

## Contents

- `architecture`
- `camera`
- `core`
- `data`
- `diagnostics`
- `doll`
- `environment`
- `formats`
- `lib.rs`
- `renderers`
- `shaders`
- `spatial`
- `streaming`
- `symbology`
- `terrain`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
