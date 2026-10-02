//! Role: pass.
//! Position: `doll/renderer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::doll::renderer::lifecycle_1::INSTANCE_STRIDE;

/// Doll pass.
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
                    r: crate::doll::scene::instances::CLEAR_COLOR[0],
                    g: crate::doll::scene::instances::CLEAR_COLOR[1],
                    b: crate::doll::scene::instances::CLEAR_COLOR[2],
                    a: crate::doll::scene::instances::CLEAR_COLOR[3],
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

/// Draw doll.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_doll(
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    cube_vbuf: &wgpu::Buffer,
    cube_ibuf: &wgpu::Buffer,
    cube_index_count: u32,
    cyl_vbuf: &wgpu::Buffer,
    cyl_ibuf: &wgpu::Buffer,
    cyl_index_count: u32,
    inst_buf: &wgpu::Buffer,
    n_cube: u32,
    n_cyl: u32,
) {
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);

    pass.set_vertex_buffer(0, cube_vbuf.slice(..));
    pass.set_vertex_buffer(1, inst_buf.slice(..u64::from(n_cube) * INSTANCE_STRIDE));
    pass.set_index_buffer(cube_ibuf.slice(..), wgpu::IndexFormat::Uint16);
    pass.draw_indexed(0..cube_index_count, 0, 0..n_cube);

    if n_cyl > 0 {
        pass.set_vertex_buffer(0, cyl_vbuf.slice(..));
        pass.set_vertex_buffer(1, inst_buf.slice(u64::from(n_cube) * INSTANCE_STRIDE..));
        pass.set_index_buffer(cyl_ibuf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..cyl_index_count, 0, 0..n_cyl);
    }
}
