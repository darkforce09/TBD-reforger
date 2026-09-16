//! Role: vector lines.
//! Position: `renderers/primitives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::lane_role_from_u32;
use crate::renderers::batching::batch::Batch;
use crate::renderers::batching::batch::BatchPayload;

use crate::renderers::batching::scene::ANCHOR;
use wasm_bindgen::prelude::*;

/// Poly lane.
pub(crate) struct PolyLane {
    /// Verts.
    pub(crate) verts: wgpu::Buffer,

    /// Indices.
    pub(crate) indices: wgpu::Buffer,

    /// Index count.
    pub(crate) index_count: u32,

    /// Item count.
    #[allow(dead_code)]
    pub(crate) item_count: u32,
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload polygon mesh.
    pub fn upload_polygon_mesh(
        &mut self,
        role: u32,
        positions: &[f32],
        colors: &[f32],
        indices: &[u32],
        item_count: u32,
        visible: bool,
    ) {
        self.polygon_lane_uploads += 1;
        let Some(role_enum) = lane_role_from_u32(role) else {
            return;
        };
        let n_verts = positions.len() / 2;
        if indices.is_empty()
            || n_verts == 0
            || colors.len() < n_verts * 4
            || !positions.len().is_multiple_of(2)
        {
            self.remove_lane(role_enum);
            self.set_vector_stat(role_enum, 0);
            return;
        }
        let mut verts = Vec::with_capacity(n_verts);
        for i in 0..n_verts {
            verts.push(crate::renderers::batching::lanes::LineVertex {
                pos: [
                    (f64::from(positions[i * 2]) - ANCHOR[0]) as f32,
                    (f64::from(positions[i * 2 + 1]) - ANCHOR[1]) as f32,
                ],
                color: [
                    colors[i * 4],
                    colors[i * 4 + 1],
                    colors[i * 4 + 2],
                    colors[i * 4 + 3],
                ],
            });
        }
        use wgpu::util::DeviceExt;
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("polygon-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("polygon-indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let index_count = indices.len() as u32;
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            Batch {
                role: role_enum,
                visible,
                payload: BatchPayload::Polygon(PolyLane {
                    verts: vbuf,
                    indices: ibuf,
                    index_count,
                    item_count,
                }),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload strip tris.
    pub fn upload_strip_tris(&mut self, role: u32, packed: &[f32], item_count: u32, visible: bool) {
        self.strip_lane_uploads += 1;
        const STRIDE: usize = 6;
        let Some(role_enum) = lane_role_from_u32(role) else {
            return;
        };
        if packed.is_empty() || !packed.len().is_multiple_of(STRIDE) {
            self.remove_lane(role_enum);
            self.set_vector_stat(role_enum, 0);
            return;
        }
        let n_verts = packed.len() / STRIDE;
        let mut verts = Vec::with_capacity(n_verts);
        for c in packed.chunks_exact(STRIDE) {
            verts.push(crate::renderers::batching::lanes::LineVertex {
                pos: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                color: [c[2], c[3], c[4], c[5]],
            });
        }

        let indices: Vec<u32> = (0..n_verts as u32).collect();
        use wgpu::util::DeviceExt;
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("strip-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("strip-indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let index_count = indices.len() as u32;
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            Batch {
                role: role_enum,
                visible,
                payload: BatchPayload::Polygon(PolyLane {
                    verts: vbuf,
                    indices: ibuf,
                    index_count,
                    item_count,
                }),
            },
        );
    }
}
