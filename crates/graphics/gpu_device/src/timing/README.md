# GPU frame timing

The timestamp-query timer of one render pass: `GpuTimer` measures the pass a renderer brackets
with its query set and keeps the last time read back.

## Contents

```text
crates/graphics/gpu_device/src/timing/
├── gpu_timer.rs  `GpuTimer`: the query set, resolve and read buffers, period and last sample (WebAssembly)
└── mod.rs        the module tree
```

## How it works

A renderer creates a `GpuTimer` when its `GpuContext` has timestamp queries enabled: a two-entry
timestamp query set, a 16-byte resolve buffer, a 16-byte map-read buffer and the queue's
timestamp period, all private. While `readback_in_flight()` is false, the render pass writes a
timestamp at its start and end into `query_set()`, `gpu_frame`'s `frame::present::submit`
resolves them into `resolve_buffer()` and copies them into `read_buffer()`, and `kick_readback`
maps it and stores (end − start) × period / 10⁶. `last_sample_ms()` is `None` before the first
readback lands and after a failed mapping, so a readout shows no reading rather than an old
number.

## Boundaries

- Depends on: `crate::buffers::readback::ReadbackLane`; `wgpu` in the WebAssembly build.
- Used by: the map engine's render engine (its boot creates the timer, its frame writes and
  resolves the timestamps, its statistics read `last_sample_ms()`), which imports it from the
  crate root.
- Rules: one readback at a time: `kick_readback` returns without mapping while its lane is in
  flight, and the renderer writes timestamps only when it is not, so the read buffer is never
  mapped twice.
