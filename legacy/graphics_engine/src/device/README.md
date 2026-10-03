# GPU resource ownership

The graphics engine's long-lived GPU buffer bookkeeping. It holds allocation and mapping
arithmetic only and never knows what the bytes depict.

## Contents

```text
legacy/graphics_engine/src/device/
├── buffers/  per-lane pooled instance buffers and one-at-a-time readback fences
└── mod.rs    the module tree
```

## How it works

The folder has one child. `buffers/` sizes and reuses the persistent vertex buffers a caller
writes lane by lane, and guards the `map_async` cycle of a readback buffer. Adapter, device,
queue and surface creation do not live here: the map engine's `RenderEngine` creates them
(`legacy/map_engine/src/frame/boot.rs`) and passes `wgpu::Device` and `wgpu::Queue`
references into this crate's functions.

## Public surface

- `device::buffers::pool`: `LanePool`, `WriteOutcome` and `grow_capacity`.
- `device::buffers::readback`: `ReadbackLane`.

## Boundaries

- Depends on: `std`, and `wgpu` in the WebAssembly build.
- Used by: `map_engine`, which re-exports `device::buffers` once, as
  `crate::frame::buffers` in `legacy/map_engine/src/frame/mod.rs`.
- Rules: the map engine names `graphics_engine::device` only at that re-export
  (`cargo xtask verify engine-layers`, rule 3b, whose pin `RULE3B_PIN` in
  `tools/foundation/repository_laws/src/engine_layers/rules.rs` counts it).
