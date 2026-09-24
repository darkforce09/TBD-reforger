# Sprite frustum culling

Drops packed 20-byte sprite instances that fall outside a view rectangle, twice over: a CPU
reference in plain Rust and a WebGPU compute pass that must agree with it.

## Contents

```text
apps/website/graphics-engine/src/draw/cull/
├── compute.rs  `IconComputeCull`: per-lane compute culling into an indirect draw (WebAssembly)
├── mod.rs      the module tree
├── oracle.rs   the CPU reference: intersection, compaction, counts, 32-byte packing
└── tests/      unit tests that hold the CPU reference and the compute pass to one answer
```

## How it works

A sprite instance is 20 bytes (`ICON_STRIDE`): position, size, yaw, cell index and tint, the
layout of `crate::draw::instances::IconInstance`. A frustum is `[min_x, min_y, max_x, max_y]` in
the same anchor-relative metres. `icon_intersects_frustum` keeps a sprite whose square of side
`size` touches the rectangle, edges included.

`oracle.rs` is the reference. `compact_icons_cpu` copies the survivors in their original order,
`count_icons_in_frustum` only counts them, and `gpu_workgroup_visible_count` reproduces the count
the shader's workgroup reduction produces. `pack_icon_storage32` and `unpack_icon_storage32`
convert between the 20-byte vertex records and the 32-byte storage records the compute shader
reads.

`compute.rs` keeps one `LaneGpu` per caller lane id, each with its own source, destination,
counter, indirect-argument, parameter and readback buffers. `upload_lane` packs a lane's sprites
into storage records and grows its buffers when needed. `encode_cull` then, for every lane,
clears the counter, dispatches `cs_icon_cull` from `crate::shaders::SHADER_WGSL` in workgroups of
`CULL_WORKGROUP` (64), and copies the survivor count into the 16-byte `draw_indirect` arguments
(`INDIRECT_STRIDE`), so the draw needs no CPU round trip. With the debug readout on, it also
copies the count into a readback buffer and computes the CPU count for comparison
(`cpu_count_for_encode`); `kick_readback` maps those buffers.

Two oracle functions read source text rather than data: `shader_reduce_barrier_before_atomic`
reads `shader.wgsl` and `cull_params_is_per_lane` reads `compute.rs`, so the file names
`compute.rs` and `oracle.rs` must stay as they are.

## Boundaries

- Depends on: `bytemuck`; `wgpu` in the WebAssembly build; `crate::shaders::SHADER_WGSL`, which
  holds `cs_icon_cull`.
- Used by: `website-map-engine`, which re-exports `oracle` and `compute` in
  `apps/website/map-engine/src/frame/mod.rs`; `RenderEngine` builds an `IconComputeCull` in
  `apps/website/map-engine/src/frame/boot.rs`, and the compute-cull readback probe in
  `apps/website/map-engine/src/diagnostics/readback/compute_cull.rs` checks it against the oracle.
- Rules: the compute pass and the CPU reference return the same count
  (`t938_3_gpu_visible_count_equals_cpu_on_fixture`, `class_r_1k_random_frusta_count_stable`);
  compaction keeps order (`class_r_compact_preserves_order_and_count`); every lane binds its own
  parameter buffer (`per_lane_cull_params_not_shared`); the storage packing round-trips
  (`storage32_roundtrip`).
