# frame/tests

Tests for the packet boundary — the module that speaks `website-graphics-engine`'s frame
vocabulary and hands it one packet per frame.

## Contents

- `damage_discipline.rs` — the pin for rule 3 of section 2C in the
  [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md): `render()` refuses an
  undamaged frame, every lane mutation marks damage, and the packet borrows `RenderEngine`'s
  persistent batch list rather than rebuilding one per frame.

## Boundaries

`RenderEngine` is `cfg(all(target_arch = "wasm32", feature = "render"))` and every method on it
needs a `wgpu::Device`, so nothing here constructs one. These are source-text pins over
`frame/{lifecycle,encode,engine}.rs` — the idiom `overlay/tests/tests/draw_order_t808_*` uses —
and they fail loudly rather than silently when a pinned function is renamed away. The
`RenderDamage` state machine itself is tested where it lives, in
`website-graphics-engine/src/frame/tests/damage_tests.rs`.
