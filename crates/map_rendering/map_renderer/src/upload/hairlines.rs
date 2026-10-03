//! **Role:** the hairline segment lanes, by role id, and the mission connection lines.
//! **Position:** the map renderer's upload belts; the asset sink forwards the terrain and forest
//! hairline uploads, and the Mission Creator's document host binds the connection lines.
//! **Signals & state:** the hairline lanes' batches and their vector counts.
//! **Invariants:** a hairline lane is six floats per vertex moved to the scene anchor; malformed
//! input removes the lane and zeroes its count.

use crate::engine::RenderEngine;
use gpu_frame::draw::lines;
use gpu_frame::frame::{DrawBatch, DrawPayload};
use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use map_draw_lanes::lane_roles::lane_role_from_u32;
use renderer_core::packet_bindings;

impl RenderEngine {
    /// Upload hairline segments (six floats per vertex) into `role`'s lane; an id that names no
    /// lane is ignored.
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

impl RenderEngine {
    /// A typed API rather than a `role_id`, for the reason `markers_bind` / `comments_bind` are: the lane has exactly one feeder (`mission_editor`'s document-fed rebind), and an upload id would open a second, unpinned door onto it. Colour lives in the caller's packing so the SELECTED edge can be tinted without the engine learning what a connection is.
    pub fn connections_bind(&mut self, packed: &[f32], item_count: u32) {
        self.upload_hairline_lane(LaneRole::MissionConnections, packed, item_count, true);
    }
}

impl RenderEngine {
    /// Upload hairline segments into `role_enum`'s lane, or remove the lane on malformed input.
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
        let stream = lines::upload_hairlines(self.gpu.device(), "hairline-verts", ANCHOR, packed);
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            DrawBatch {
                lane: lane_id(role_enum),
                visible,
                pipeline: packet_bindings::PIPE_LINE,
                payload: DrawPayload::Lines(stream),
            },
        );
    }
}
