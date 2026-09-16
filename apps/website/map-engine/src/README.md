# Graphics engine modules

Graphics modules and native regression suites for rendering contracts and camera parity.

## Contents

- `architecture`
- `camera`
- `core`
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

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
