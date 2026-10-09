# Map renderer tests

The native test of the map renderer: the surface size policy.

## Contents

```text
crates/map_rendering/map_renderer/src/tests/
└── surface_size_tests.rs       the surface size policy: JavaScript's rounding, at least one pixel, non-positive sizes refused
```

## Boundaries

`RenderEngine` is `wasm32` only and every method on it needs a `wgpu::Device`, so nothing here
constructs one. The `RenderDamage` state machine is tested where it lives, in
`crates/graphics/render_primitives/src/frame/tests/damage_tests.rs`. The surface size test reads
plain values and runs natively.
