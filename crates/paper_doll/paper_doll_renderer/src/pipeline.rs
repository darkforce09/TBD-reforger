//! The paper doll's shader program, render pipeline and depth target.
//!
//! **Role:** compiles `shaders/doll.wgsl` with its one uniform binding into a [`DollProgram`],
//! builds the doll's render pipeline for a colour format ([`DollProgram::pipeline`]), its uniform
//! buffer and bind group ([`DollProgram::uniforms`]), and the `Depth32Float` target
//! ([`create_depth`]).
//! **Position:** inside `paper_doll_renderer`; the renderer builds its program and pipeline once
//! for the surface format, and the self-check builds a second pipeline for its `Rgba8Unorm` probe
//! target from a clone of the same program.
//! **Signals & state:** none of its own; the GPU objects it returns belong to the caller.
//! **Invariants:** the uniform is [`UNIFORM_SIZE`] bytes (the `mvp` matrix and a `params` vector,
//! `DollUniforms` in `doll.wgsl`); vertices are 24 bytes (position, normal) at locations 0 and 1,
//! instances [`INSTANCE_STRIDE_BYTES`] (four matrix columns and a colour) at locations 2 to 6; the
//! pipeline has no culling and no blending and depth-tests with `Less`.

use crate::instance_packing::INSTANCE_STRIDE;

/// The bytes of the doll's uniform: a 4x4 `f32` matrix and a `params` vector of 4 `f32`.
pub(crate) const UNIFORM_SIZE: u64 = 80;

/// The bytes of one part in the instance buffer, as the vertex layout and the draws need it.
pub(crate) const INSTANCE_STRIDE_BYTES: u64 = INSTANCE_STRIDE as u64;

/// The doll's compiled shader and the layouts every doll pipeline and uniform bind group share.
#[derive(Clone)]
pub(crate) struct DollProgram {
    /// The compiled `doll.wgsl`.
    pub(crate) shader: wgpu::ShaderModule,

    /// The layout of the one uniform binding at group 0.
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,

    /// The pipeline layout over that bind group layout.
    pub(crate) pipeline_layout: wgpu::PipelineLayout,
}

impl DollProgram {
    /// Compile the doll shader and build its layouts on `device`.
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("doll3d"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/doll.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("doll-uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(UNIFORM_SIZE),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("doll3d"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        Self {
            shader,
            bind_group_layout,
            pipeline_layout,
        }
    }

    /// The doll's render pipeline writing colour `format`.
    pub(crate) fn pipeline(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        create_doll_pipeline(device, &self.pipeline_layout, &self.shader, format)
    }

    /// A uniform buffer of [`UNIFORM_SIZE`] bytes and the bind group that binds it, both labelled
    /// `label`.
    pub(crate) fn uniforms(
        &self,
        device: &wgpu::Device,
        label: &str,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        (buffer, bind_group)
    }
}

/// The doll's render pipeline: instanced parts, no culling, no blending, depth test `Less`.
fn create_doll_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("doll3d"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_doll"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 24,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                },
                wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_STRIDE_BYTES,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4,
                        6 => Float32x4
                    ],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_doll"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),

        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// A `Depth32Float` depth target of `w` × `h` texels, each side at least 1.
pub(crate) fn create_depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("doll-depth"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}
