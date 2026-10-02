# Sprite frustum culling, CPU reference

The CPU reference of sprite frustum culling: which packed 20-byte sprite instances survive a view
rectangle, in what order, and how many the GPU compute kernel's workgroup reduction counts. The
graphics engine's compute pass is checked against it.

## Contents

```text
crates/graphics/render_primitives/src/draw/cull/
├── mod.rs     the module tree
├── oracle.rs  the CPU reference: intersection, compaction, counts, 32-byte packing
└── tests/     unit tests that hold the reference and the modelled reduce to one answer
```

## How it works

A sprite instance is 20 bytes (`ICON_STRIDE`): position, size, yaw, cell index and tint, the
layout of `crate::draw::instances::IconInstance`. A frustum is `[min_x, min_y, max_x, max_y]` in
the same anchor-relative metres. `icon_intersects_frustum` keeps a sprite whose square of side
`size` touches the rectangle, edges included.

`compact_icons_cpu` copies the survivors in their original order, `count_icons_in_frustum` only
counts them, and `gpu_workgroup_visible_count` reproduces the count the shader's workgroup
reduction produces. `shader_reduce_barrier_before_atomic` reads `shaders/shader.wgsl` to tell
whether the kernel's barrier precedes its atomic add, which decides that count.
`pack_icon_storage32` and `unpack_icon_storage32` convert between the 20-byte vertex records and
the 32-byte storage records the compute shader reads.

## Boundaries

- Depends on: `bytemuck`; the WGSL source in `crate::shaders`, read by relative path.
- Used by: the graphics engine's compute cull (`legacy/graphics_engine/src/draw/cull/`), which
  packs with it, compares its debug count against it and forwards it as `draw::cull::oracle`.
- Rules: the modelled reduce and the CPU count agree
  (`t938_3_gpu_visible_count_equals_cpu_on_fixture`, `class_r_1k_random_frusta_count_stable`);
  compaction keeps order (`class_r_compact_preserves_order_and_count`); the storage packing
  round-trips (`storage32_roundtrip`).
