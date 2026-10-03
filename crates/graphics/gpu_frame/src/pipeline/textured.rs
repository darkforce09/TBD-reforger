//! Role: the textured rect pipelines, including the one with a density fragment entry point.
//! Position: `pipeline` in the GPU frame crate; the map engine's render engine builds its pipelines with these at boot.
//! Signals & state: none; each constructor builds a `wgpu::RenderPipeline` on the caller's device and returns it.
//! Invariants: every pipeline compiles against the caller's shader module (from `create_render_shader`) and pipeline layout; its vertex buffer layouts match the `render_primitives` byte layouts and the WGSL vertex inputs; no depth or stencil state, single-sampled.

/// Create textured pipeline.
pub fn create_textured_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    create_textured_pipeline_with_fs(
        device,
        layout,
        shader,
        format,
        "fs_textured",
        "textured-quad",
    )
}

/// Create the density pipeline: a textured rect whose fragment entry tints a sampled density
/// raster.
pub fn create_density_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    create_textured_pipeline_with_fs(
        device,
        layout,
        shader,
        format,
        "fs_forest_density",
        "density-textured",
    )
}

/// Create textured pipeline with fs.
pub fn create_textured_pipeline_with_fs(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    fs_entry: &str,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_textured"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                },
                wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x4],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
