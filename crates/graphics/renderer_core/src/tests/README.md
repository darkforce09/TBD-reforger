# Renderer core tests

The native unit tests of the renderer contracts' CPU parts: the frame packet's binding ids, the
render statistics, the statistics JSON writer and the frame hooks.

## Contents

```text
crates/graphics/renderer_core/src/tests/
└── packet_bindings_tests.rs  dense pipeline ids, dense fixed bind ids below the texture base, per-lane texture slots
```

## Boundaries

- Depends on: `crate::frame_hook`, `crate::packet_bindings`, `crate::render_stats` and
  `crate::stats_json`, through the `#[path = "tests/…"]` modules of those files;
  `render_primitives::frame::ids`.
- Used by: `cargo test -p renderer_core`.
- Rules: native only; no GPU handle is created.
