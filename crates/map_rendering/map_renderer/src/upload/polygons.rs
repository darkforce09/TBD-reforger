//! **Role:** the vector lanes' indexed polygon meshes and triangle strips, by role id.
//! **Position:** the map renderer's upload belts; the asset sink forwards the terrain loaders'
//! sea, landcover, road and forest uploads.
//! **Signals & state:** the vector lanes' batches, their vector counts and the polygon and strip
//! upload counters.
//! **Invariants:** a mesh crosses to the GPU relative to the scene anchor; malformed input removes
//! the lane and zeroes its count.

use crate::engine::RenderEngine;
use gpu_frame::draw::polygons;
use gpu_frame::frame::{DrawBatch, DrawPayload};
use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::lane_id;
use map_draw_lanes::lane_roles::lane_role_from_u32;
use renderer_core::packet_bindings;

impl RenderEngine {
    /// Upload an indexed polygon mesh (positions, RGBA colours, indices) into `role`'s lane; an
    /// id that names no lane is ignored.
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
        let mesh = polygons::upload_polygon_mesh(
            self.gpu.device(),
            ANCHOR,
            positions,
            colors,
            indices,
            item_count,
        );
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            DrawBatch {
                lane: lane_id(role_enum),
                visible,
                pipeline: packet_bindings::PIPE_POLYGON,
                payload: DrawPayload::Indexed(mesh),
            },
        );
    }
}

impl RenderEngine {
    /// Upload a triangle strip mesh (six floats per vertex) into `role`'s lane; an id that names
    /// no lane is ignored.
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
        let mesh = polygons::upload_strip_mesh(self.gpu.device(), ANCHOR, packed, item_count);
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            DrawBatch {
                lane: lane_id(role_enum),
                visible,
                pipeline: packet_bindings::PIPE_POLYGON,
                payload: DrawPayload::Indexed(mesh),
            },
        );
    }
}
