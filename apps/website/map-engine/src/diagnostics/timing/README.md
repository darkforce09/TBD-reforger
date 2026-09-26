# Frame timing

The clocks the render engine times itself with: `now_ms` and `perf_now_ms` for CPU time, and
`GpuTimer`, which measures the main render pass with timestamp queries when the adapter offers
them.

## Contents

```text
apps/website/map-engine/src/diagnostics/timing/
├── gpu.rs  `now_ms`, `perf_now_ms` and `GpuTimer`, the main pass's timestamp queries and readback
└── mod.rs  the module tree
```

## How it works

`gpu.rs` compiles only for wasm32 with the `render` feature. `now_ms` reads `Date.now()`;
`perf_now_ms` reads `performance.now()` and falls back to `now_ms`. `RenderEngine::create` builds a
`GpuTimer` only when the adapter supports `TIMESTAMP_QUERY`: a two-entry timestamp query set, a
16-byte resolve buffer, a 16-byte map-read buffer and the queue's timestamp period. When no readback
is in flight, the frame's render pass writes a timestamp at its start and end, the submit resolves
them and copies them into the read buffer, and `kick_readback` maps it and stores
(end − start) × period / 10⁶ in `last_ms`, which `stats()` reports as `gpu_frame_ms`. The
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s canvas boot and the debug
benches call `disable_frame_timing`, which drops the timer, so their frames carry no timestamps.

## Boundaries

- Depends on: `crate::frame::buffers::readback::ReadbackLane`, the graphics engine's readback
  in-flight state that `crate::frame` re-exports; `js-sys`, `web-sys` and `wgpu`.
- Used by: `crate::frame` (`boot.rs` creates the timer, `engine.rs` holds it, `encode.rs` and
  `lifecycle.rs` write and resolve the timestamps, and `lifecycle.rs` times each frame with
  `perf_now_ms`); `crate::diagnostics::bench` (both clocks and the timer's last sample).
- Rules: one readback at a time: `kick_readback` returns without mapping while its lane is in
  flight, and the frame writes timestamps only when it is not, so the read buffer is never mapped
  twice.
