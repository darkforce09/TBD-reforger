# shaders

WGSL programs for the map and mannequin pipelines; bindings and vertex layouts match their Rust owners.

## Contents

- `doll.wgsl`
- `shader.wgsl`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
