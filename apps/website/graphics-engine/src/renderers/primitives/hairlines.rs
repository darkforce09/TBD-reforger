//! Role: hairlines.
//! Position: `renderers/primitives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::core::pipeline::draw_order::lane_role_from_u32;
use crate::renderers::batching::batch::Batch;
use crate::renderers::batching::batch::BatchPayload;

use crate::renderers::batching::scene::ANCHOR;
use wasm_bindgen::prelude::*;

/// Line lane.
pub(crate) struct LineLane {
    /// Verts.
    pub(crate) verts: wgpu::Buffer,

    /// Count.
    pub(crate) count: u32,
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload hairline segments.
    pub fn upload_hairline_segments(
        &mut self,
        role: u32,
        packed: &[f32],
        item_count: u32,
        visible: bool,
    ) {
        let Some(role_enum) = lane_role_from_u32(role) else {
            return;
        };
        self.upload_hairline_lane(role_enum, packed, item_count, visible);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// A typed API rather than a `role_id`, for the reason `markers_bind` / `comments_bind` are: the lane has exactly one feeder (`mission_editor`'s document-fed rebind), and an upload id would open a second, unpinned door onto it. Colour lives in the caller's packing so the SELECTED edge can be tinted without the engine learning what a connection is.
    pub fn connections_bind(&mut self, packed: &[f32], item_count: u32) {
        self.upload_hairline_lane(LaneRole::MissionConnections, packed, item_count, true);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload hairline lane.
    pub(crate) fn upload_hairline_lane(
        &mut self,
        role_enum: LaneRole,
        packed: &[f32],
        item_count: u32,
        visible: bool,
    ) {
        const STRIDE: usize = 6;
        if packed.is_empty() || !packed.len().is_multiple_of(STRIDE) {
            self.remove_lane(role_enum);
            self.set_vector_stat(role_enum, 0);
            return;
        }
        let mut verts = Vec::with_capacity(packed.len() / STRIDE);
        for c in packed.chunks_exact(STRIDE) {
            verts.push(crate::renderers::batching::lanes::LineVertex {
                pos: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                color: [c[2], c[3], c[4], c[5]],
            });
        }
        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("hairline-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = verts.len() as u32;
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            Batch {
                role: role_enum,
                visible,
                payload: BatchPayload::Lines(LineLane { verts: buf, count }),
            },
        );
    }
}
