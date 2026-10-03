# frame/tests

Tests for the packet boundary — the module that speaks `graphics_engine`'s frame
vocabulary and hands it one packet per frame.

## Contents

- `damage_discipline.rs` — the pin for rule 3 of section 2C in the
  [engine boundary rules](/documentation/standards/engine_boundary_rules.md): `render()` refuses an
  undamaged frame, every lane mutation marks damage, and the packet borrows `RenderEngine`'s
  persistent batch list rather than rebuilding one per frame.
- `lane_bind_source_pins/` — source-text pins of the comment, connection and symbology bind
  functions: what each uploads and that none touches the pick bridge.

## Boundaries

`RenderEngine` is `cfg(all(target_arch = "wasm32", feature = "render"))` and every method on it
needs a `wgpu::Device`, so nothing here constructs one. These are source-text pins over
`frame/{lifecycle,encode,engine}.rs` — the idiom `lane_bind_source_pins/` uses —
and they fail loudly rather than silently when a pinned function is renamed away. The
`RenderDamage` state machine itself is tested where it lives, in
`crates/graphics/render_primitives/src/frame/tests/damage_tests.rs`.
