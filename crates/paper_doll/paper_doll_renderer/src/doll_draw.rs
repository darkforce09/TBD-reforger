//! The paper doll's GPU meshes, its render pass and its two instanced draws.
//!
//! **Role:** uploads the unit cube and the 16-segment cylinder of `paper_doll_scene` once
//! ([`DollMeshes::upload`]), opens the doll's render pass clearing colour and depth
//! ([`doll_pass`]), and draws every cube part then every cylinder part from one instance buffer
//! ([`draw_doll`]).
//! **Position:** inside `paper_doll_renderer`; the frame render draws into the swapchain image,
//! the self-check into its `Rgba8Unorm` probe target, both with the meshes the renderer uploaded.
//! **Signals & state:** none of its own; the buffers belong to the renderer.
//! **Invariants:** the pass clears to `CLEAR_COLOR` and depth 1.0 and discards depth afterwards;
//! the cube instances occupy the instance buffer's start and the cylinder instances follow them,
//! [`INSTANCE_STRIDE_BYTES`] bytes each.

use crate::pipeline::INSTANCE_STRIDE_BYTES;
use paper_doll_scene::part_meshes::{mesh_cube, mesh_cylinder};
use paper_doll_scene::soldier_parts::CLEAR_COLOR;

/// The side count of the uploaded cylinder.
const CYLINDER_SEGMENTS: usize = 16;

/// One uploaded unit mesh: its interleaved vertices, its `u16` indices and their count.
#[derive(Clone)]
pub(crate) struct MeshBuffers {
    /// Position and normal, 24 bytes a vertex.
    pub(crate) vertices: wgpu::Buffer,

    /// `u16` triangle indices.
    pub(crate) indices: wgpu::Buffer,

    /// The number of indices.
    pub(crate) index_count: u32,
}

impl MeshBuffers {
    /// Upload `verts` and `idx` into new buffers labelled `label`.
    fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        verts: &[f32],
        idx: &[u16],
    ) -> Self {
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (verts.len() * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertices, 0, bytemuck::cast_slice(verts));
        let indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (idx.len() * 2) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&indices, 0, bytemuck::cast_slice(idx));
        Self {
            vertices,
            indices,
            index_count: u32::try_from(idx.len()).expect("index count fits u32"),
        }
    }
}

/// The two unit meshes every doll part scales.
#[derive(Clone)]
pub(crate) struct DollMeshes {
    /// The unit cube.
    pub(crate) cube: MeshBuffers,

    /// The unit cylinder of [`CYLINDER_SEGMENTS`] sides.
    pub(crate) cylinder: MeshBuffers,
}

impl DollMeshes {
    /// Upload the unit cube and the unit cylinder.
    pub(crate) fn upload(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let (cube_v, cube_i) = mesh_cube();
        let (cyl_v, cyl_i) = mesh_cylinder(CYLINDER_SEGMENTS);
        Self {
            cube: MeshBuffers::upload(device, queue, "doll-cube", &cube_v, &cube_i),
            cylinder: MeshBuffers::upload(device, queue, "doll-cyl", &cyl_v, &cyl_i),
        }
    }
}

/// An instance buffer packed by `pack_instances` and how many of its parts each mesh draws.
pub(crate) struct InstanceDraw<'a> {
    /// The packed instance stream, cubes first.
    pub(crate) buffer: &'a wgpu::Buffer,

    /// The cube parts at the start of the buffer.
    pub(crate) cube_instances: u32,

    /// The cylinder parts after the cubes.
    pub(crate) cylinder_instances: u32,
}

/// Open the doll's render pass into `color`, clearing it to `CLEAR_COLOR` and `depth` to 1.0.
pub(crate) fn doll_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    color: &'a wgpu::TextureView,
    depth: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("doll"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: CLEAR_COLOR[0],
                    g: CLEAR_COLOR[1],
                    b: CLEAR_COLOR[2],
                    a: CLEAR_COLOR[3],
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Discard,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

/// Draw every cube part, then every cylinder part, with `pipeline` and its uniform `bind_group`.
pub(crate) fn draw_doll(
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    meshes: &DollMeshes,
    instances: &InstanceDraw<'_>,
) {
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);

    let cube_bytes = u64::from(instances.cube_instances) * INSTANCE_STRIDE_BYTES;
    pass.set_vertex_buffer(0, meshes.cube.vertices.slice(..));
    pass.set_vertex_buffer(1, instances.buffer.slice(..cube_bytes));
    pass.set_index_buffer(meshes.cube.indices.slice(..), wgpu::IndexFormat::Uint16);
    pass.draw_indexed(0..meshes.cube.index_count, 0, 0..instances.cube_instances);

    if instances.cylinder_instances > 0 {
        pass.set_vertex_buffer(0, meshes.cylinder.vertices.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(cube_bytes..));
        pass.set_index_buffer(meshes.cylinder.indices.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(
            0..meshes.cylinder.index_count,
            0,
            0..instances.cylinder_instances,
        );
    }
}
