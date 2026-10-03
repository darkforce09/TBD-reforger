# Sprite frustum culling, GPU side

Drops packed 20-byte sprite instances that fall outside a view rectangle in a WebGPU compute pass,
which must agree with the CPU reference `render_primitives::draw::cull::oracle`
(`crates/graphics/render_primitives/src/draw/cull/`).

## Contents

```text
crates/graphics/gpu_frame/src/draw/cull/
├── compute.rs  `IconComputeCull`: per-lane compute culling into an indirect draw (WebAssembly)
├── mod.rs      the module tree
└── tests/      the source check that every lane binds its own parameter buffer
```

## How it works

`compute.rs` keeps one `LaneGpu` per caller lane id, each with its own source, destination,
counter, indirect-argument, parameter and readback buffers. `upload_lane` packs a lane's sprites
into 32-byte storage records (`oracle::pack_icon_storage32`) and grows its buffers when needed.
`encode_cull` then, for every lane, clears the counter, dispatches `cs_icon_cull` from
`render_primitives::shaders::SHADER_WGSL` in workgroups of `CULL_WORKGROUP` (64), and copies the
survivor count into the 16-byte `draw_indirect` arguments (`INDIRECT_STRIDE`), so the draw needs
no CPU round trip. With the debug readout on, it also copies the count into a readback buffer and
computes the CPU count for comparison (`oracle::cpu_count_for_encode`); `kick_readback` maps those
buffers.

`compute.rs` compiles for WebAssembly only, so `tests/compute_source_tests.rs` reads its source
text on the native target to prove that every lane writes and binds its own parameter buffer.
The file name `compute.rs` is therefore load-bearing.

## Boundaries

- Depends on: `render_primitives` (the oracle and `SHADER_WGSL`, which holds `cs_icon_cull`);
  `bytemuck`; `wgpu` in the WebAssembly build.
- Used by: `RenderEngine`, which builds an `IconComputeCull` in
  `crates/map_rendering/map_renderer/src/boot.rs`; `symbology_layers_gpu`'s icon cull
  (`crates/map_rendering/symbology_layers_gpu/src/icon_cull_gpu.rs`), which holds it; and the
  compute-cull readback probe in
  `crates/map_rendering/map_render_diagnostics/src/readback/compute_cull.rs` checks it against the oracle.
- Rules: every lane binds its own parameter buffer (`per_lane_cull_params_not_shared`); the
  compute pass returns the oracle's count (`t938_3_gpu_visible_count_equals_cpu_on_fixture` in
  `render_primitives`).
