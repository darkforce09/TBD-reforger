# GPU resource ownership

The graphics engine's long-lived GPU buffer bookkeeping. It holds allocation and mapping
arithmetic only and never knows what the bytes depict.

## Contents

```text
apps/website/graphics-engine/src/device/
├── buffers/  per-lane pooled instance buffers and one-at-a-time readback fences
└── mod.rs    the module tree
```

## How it works

The folder has one child. `buffers/` sizes and reuses the persistent vertex buffers a caller
writes lane by lane, and guards the `map_async` cycle of a readback buffer. Adapter, device,
queue and surface creation do not live here: the map engine's `RenderEngine` creates them
(`apps/website/map-engine/src/frame/boot.rs`) and passes `wgpu::Device` and `wgpu::Queue`
references into this crate's functions.

## Public surface

- `device::buffers::pool`: `LanePool`, `WriteOutcome` and `grow_capacity`.
- `device::buffers::readback`: `ReadbackLane`.

## Boundaries

- Depends on: `std`, and `wgpu` in the WebAssembly build.
- Used by: `website-map-engine`, which re-exports `device::buffers` once, as
  `crate::frame::buffers` in `apps/website/map-engine/src/frame/mod.rs`.
- Rules: the map engine names `website_graphics_engine::device` only at that re-export
  (`cargo xtask verify engine-layers`, rule 3b, whose pin in
  `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs` counts it).
