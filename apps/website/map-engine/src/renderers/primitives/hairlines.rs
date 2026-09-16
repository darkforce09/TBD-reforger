//! Role: hairlines.
//! Position: `renderers/primitives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::bindings;
use crate::core::pipeline::draw_order::LaneRole;
use crate::core::pipeline::draw_order::lane_id;
use crate::core::pipeline::draw_order::lane_role_from_u32;

use crate::renderers::batching::scene::ANCHOR;
use wasm_bindgen::prelude::*;
use website_graphics_engine::draw::lines;
use website_graphics_engine::frame::{DrawBatch, DrawPayload};

// T-0xx Phase 1D: `LineLane` is gone — it is `website_graphics_engine::frame::VertexStream`,
// and the loop that filled it is `draw::lines::upload_hairlines`. `ANCHOR` could not cross,
// so it is an argument there.

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
        let stream = lines::upload_hairlines(&self.device, "hairline-verts", ANCHOR, packed);
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            DrawBatch {
                lane: lane_id(role_enum),
                visible,
                pipeline: bindings::PIPE_LINE,
                payload: DrawPayload::Lines(stream),
            },
        );
    }
}
