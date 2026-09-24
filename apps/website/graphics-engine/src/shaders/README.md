# WGSL shaders

The graphics engine's one shader source: every vertex and fragment entry point its pipelines use,
and the compute kernel that culls sprites, embedded in the crate as a string.

## Contents

```text
apps/website/graphics-engine/src/shaders/
├── mod.rs       the module tree; `SHADER_WGSL`, the WGSL source embedded with `include_str!`
├── shader.wgsl  every render entry point and the `cs_icon_cull` compute kernel
└── tests/       unit tests that pin the text uniform block and text sampling in the source
```

## How it works

`SHADER_WGSL` is the text of `shader.wgsl`, included by a path relative to `mod.rs`, so the
source can only live inside this crate. `crate::pipeline::create_map_shader` compiles it once
into a `wgpu::ShaderModule`, and every pipeline constructor and the compute cull use that module.

| Entry points | Draws | Bind groups read |
|---|---|---|
| `vs_main`, `fs_main` | axis-aligned coloured quads | 0: camera |
| `vs_textured`, `fs_textured`, `fs_forest_density` | textured quads, north-up; a tree-count texture shaded as a canopy fill | 0: camera; 1: texture and sampler |
| `vs_line`, `fs_line` | line lists and triangle lists of `LineVertex` | 0: camera |
| `vs_building`, `fs_building` | oriented quads from a centre, half-extents and a precomputed rotation | 0: camera |
| `vs_icon`, `fs_icon` | atlas sprites with yaw and tint, from a 32-entry UV table | 0: camera; 2: atlas, sampler, `IconUniforms` |
| `vs_text`, `fs_text` | glyph cells from the text atlas grid in `TextUniforms` | 0: camera; 2: atlas, sampler, `TextUniforms` |
| `cs_icon_cull` | compute: compacts visible sprites, counts them per workgroup | 0: input, output, counter, `CullParams` |

Positions arrive in anchor-relative metres, and `Uniforms` at group 0 carries the caller's
clip-from-local matrix (`crate::frame::CameraUniform`). Group 1 belongs to a textured quad's own
texture and group 2 to an atlas, so the binding numbers never collide. `cs_icon_cull` runs in
workgroups of 64: each thread tests one sprite, the workgroup reduces its visibility bits after a
barrier, and one `atomicAdd` per workgroup reserves the output range.

## Boundaries

- Depends on: nothing; the shader's structs must match the byte layouts in `crate::draw::instances`,
  `crate::draw::geometry`, `crate::frame::camera` and `crate::text::pack`.
- Used by: `crate::pipeline` and `crate::draw::cull`, and the source-text checks in
  `crate::draw::cull::oracle` and `crate::draw` tests. `website-map-engine` reaches the shader only
  through `create_map_shader`, never through this module.
- Rules: `TextUniforms` is four `f32`s, 16 bytes, with no `vec3` padding, and `vs_text` flips V
  and reads its grid from the uniform (`g1_text_uniforms_is_16_bytes_no_vec3`,
  `g1_vs_text_has_v_flip`, `l2_vs_text_grid_from_uniform`); the sprite UV table is
  `ATLAS_GLYPH_COUNT` entries long and the cell index is clamped to it
  (`shader_uv_table_tracks_atlas_glyph_count` in `crate::draw`); `cs_icon_cull` places its barrier
  before the atomic, which `shader_reduce_barrier_before_atomic` reads from the source and
  `t938_3_gpu_visible_count_equals_cpu_on_fixture` fails without; the map engine names no
  `website_graphics_engine::shaders` path (`cargo xtask verify engine-layers`, rule 3b).
