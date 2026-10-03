# Stress scene tests

Byte-exact cases for the stress scene the benchmark's stress pool draws.

## Contents

```text
crates/map_rendering/map_render_diagnostics/src/benchmark/tests/
└── stress_scene_tests.rs  determinism, Everon domain bounds and the pinned first-instance bits of `stress_chunk`
```

## Boundaries

The cases are pure arithmetic over `render_primitives`' `QuadInstance` and run natively with
`cargo test -p map_render_diagnostics`.
