//! Role: preferences.
//! Position: `core/context` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::bindings;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;
use crate::overlay::lanes::tex_lane_role_from_u32;

use wasm_bindgen::prelude::*;
use website_graphics_engine::draw::lines;
use website_graphics_engine::frame::{DrawBatch, DrawPayload};

#[wasm_bindgen]
impl RenderEngine {
    /// Set grid.
    pub fn set_grid(&mut self, width: f64, height: f64, over_hillshade: bool, visible: bool) {
        let verts = crate::renderers::batching::lanes::grid_lines(width, height, over_hillshade);
        if verts.is_empty() {
            self.remove_lane(LaneRole::Grid);
            return;
        }
        let stream = lines::upload_line_stream(&self.device, "grid-lines", &verts);
        self.upsert_lane(
            LaneRole::Grid,
            DrawBatch {
                lane: lane_id(LaneRole::Grid),
                visible,
                pipeline: bindings::PIPE_LINE,
                payload: DrawPayload::Lines(stream),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set world layer visible.
    pub fn set_world_layer_visible(&mut self, key: &str, visible: bool) {
        let roles: &[LaneRole] = match key {
            "roads" => &[LaneRole::RoadsCasing, LaneRole::Roads],
            "forest" => &[LaneRole::ForestFill, LaneRole::ForestOutline],
            "contours" => &[LaneRole::Contours],
            "sea" => &[LaneRole::Sea],
            "airfield" => &[LaneRole::WorldAirfieldApron],
            "heights" => &[LaneRole::WorldLabels],
            "townLabels" => &[LaneRole::WorldTownLabels],
            "roadNames" => &[LaneRole::WorldRoadLabels],
            _ => return,
        };
        // T-0xx Phase 1D: the world-layer table is cartography — `"roads"` means two lanes,
        // one casing and one surface — so it stays here and is resolved to lane ids before it
        // ever reaches a batch.
        let lanes: Vec<_> = roles.iter().copied().map(lane_id).collect();
        let mut changed = false;
        for b in &mut self.batches {
            if lanes.contains(&b.lane) && b.visible != visible {
                b.visible = visible;
                changed = true;
            }
        }
        if changed {
            self.damage.mark();
        }
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Re-tint a committed lane's opacity in place + toggle its visibility (`tex_role_id::BASEMAP` / `HILLSHADE`) — no texture rebuild (L6 cheap-memo / hybrid-style dim). The tint alpha is `color[3]` at byte offset 16 of the 1-instance quad buffer.
    pub fn set_lane_opacity(&mut self, role: u32, opacity: f32, visible: bool) {
        let Some(want) = tex_lane_role_from_u32(role) else {
            debug_assert!(
                false,
                "set_lane_opacity: role must be 0 (basemap) or 1 (hillshade), got {role} — \
                 ignored. Note this is NOT the vector-lane `role_id` namespace."
            );
            return;
        };
        let color = [1.0f32, 1.0, 1.0, opacity.clamp(0.0, 1.0)];
        let want_lane = lane_id(want);
        let target = self.batches.iter_mut().find_map(|b| {
            if b.lane == want_lane {
                b.visible = visible;
                if let DrawPayload::TexturedRect { instances, .. } = &b.payload {
                    return Some(instances.buffer.clone());
                }
            }
            None
        });
        if let Some(buf) = target {
            self.queue
                .write_buffer(&buf, 16, bytemuck::cast_slice(&color));
        }
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set clear color.
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64) {
        self.clear_color = wgpu::Color { r, g, b, a: 1.0 };
    }
}
