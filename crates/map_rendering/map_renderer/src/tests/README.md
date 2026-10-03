# Map renderer tests

The tests of the map renderer: the source pins of its damage-driven frame path and of the mission
lane bind functions, the statistics JSON, the calibration bytes, the surface size policy and the
error messages.

## Contents

```text
crates/map_rendering/map_renderer/src/tests/
├── calibration_scene_tests.rs  the byte-exact pin of the two calibration quads
├── damage_discipline.rs        the damage-driven frame path pins: `render()` refuses an undamaged frame, every lane mutation marks damage, the packet borrows the persistent batch list and tables
├── engine_statistics_json.rs   the pin of `stats()`'s JSON: byte-identical to the `format!` string it replaced, keys and order fixed
├── error_tests.rs              the stable codes of the engine's error messages
├── lane_bind_source_pins/      source-text pins of the comment, connection and symbology bind functions
└── surface_size_tests.rs       the surface size policy: JavaScript's rounding, at least one pixel, non-positive sizes refused
```

## Boundaries

`RenderEngine` is `wasm32` only and every method on it needs a `wgpu::Device`, so nothing here
constructs one. The damage pins (rule 3 of section 2C in the
[crate boundary rules](/documentation/standards/crate_boundary_rules.md)) are source-text pins
over `lifecycle.rs`, `encode.rs`, `engine.rs` and `lane_sinks/engine_lane_sink.rs`, the idiom
`lane_bind_source_pins/` uses, and they fail loudly rather than silently when a pinned function is
renamed away. The `RenderDamage` state machine itself is tested where it lives, in
`crates/graphics/render_primitives/src/frame/tests/damage_tests.rs`. The statistics pin builds the
report as plain data and the calibration, surface size and error tests read plain values; all run
natively.
