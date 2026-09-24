# Render pipeline constructors

The functions that build the graphics engine's render pipelines and the one shader module they
all compile against. Each constructor pairs a vertex layout with its entry points in
`crate::shaders::SHADER_WGSL`; the caller supplies the device, the pipeline layout and the colour
format.

## Contents

```text
apps/website/graphics-engine/src/pipeline/
├── building.rs  `create_building_pipeline`: oriented 40-byte quads
├── icon.rs      atlas-sprite pipelines for 20-byte instances and 32-byte culled records
├── mod.rs       the module tree, and `create_map_shader`, which compiles `SHADER_WGSL` once
├── quad.rs      `create_quad_pipeline`: axis-aligned 32-byte quads, opaque
├── text.rs      `create_text_pipeline`: glyph runs in the 20-byte sprite layout
├── textured.rs  textured quads, forest-density shading, any other textured fragment entry
└── vector.rs    `create_line_pipeline` and `create_polygon_pipeline`: lines and triangles
```

## How it works

The caller compiles the shader once with `create_map_shader(device)` and passes the module, a
`wgpu::PipelineLayout` whose bind-group layouts it created, and the colour target format to each
constructor. Every file compiles for WebAssembly only.

| Constructor | Vertex buffers (stride in bytes) | Topology | Blend | Entry points |
|---|---|---|---|---|
| `create_quad_pipeline` | unit quad (8), instance (32) | triangle strip | none | `vs_main`, `fs_main` |
| `create_textured_pipeline` | unit quad (8), instance (32) | triangle strip | alpha | `vs_textured`, `fs_textured` |
| `create_forest_density_pipeline` | unit quad (8), instance (32) | triangle strip | alpha | `vs_textured`, `fs_forest_density` |
| `create_textured_pipeline_with_fs` | unit quad (8), instance (32) | triangle strip | alpha | `vs_textured`, the caller's fragment entry |
| `create_line_pipeline` | vertex (24) | line list | alpha | `vs_line`, `fs_line` |
| `create_polygon_pipeline` | vertex (24) | triangle list | alpha | `vs_line`, `fs_line` |
| `create_building_pipeline` | unit quad (8), instance (40) | triangle strip | alpha | `vs_building`, `fs_building` |
| `create_icon_pipeline` | unit quad (8), instance (20) | triangle strip | alpha | `vs_icon`, `fs_icon` |
| `create_icon_pipeline_storage32` | unit quad (8), instance (32) | triangle strip | alpha | `vs_icon`, `fs_icon` |
| `create_text_pipeline` | unit quad (8), instance (20) | triangle strip | alpha | `vs_text`, `fs_text` |

No pipeline has a depth or stencil state. `create_icon_pipeline_storage32` reads the 32-byte
records that the compute cull in `crate::draw::cull` writes, so culled sprites draw straight from
the compute pass's output buffer.

## Boundaries

- Depends on: `wgpu`; `crate::shaders::SHADER_WGSL`. No Leptos and no application state.
- Used by: `website-map-engine`, which re-exports this module once as `crate::frame::pipelines`
  (`apps/website/map-engine/src/frame/mod.rs`); `RenderEngine` builds its pipelines through it in
  `apps/website/map-engine/src/frame/boot.rs`, and the readback probes and probe runner in
  `apps/website/map-engine/src/diagnostics/` build offscreen copies.
- Rules: each vertex stride matches its layout in `crate::draw::instances` or
  `crate::draw::geometry`, and each entry point exists in `shader.wgsl` (checked when the pipeline
  is created in the browser); the map engine names this module only at its one re-export
  (`cargo xtask verify engine-layers`, rule 3b).
