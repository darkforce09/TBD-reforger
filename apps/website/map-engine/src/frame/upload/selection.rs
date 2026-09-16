//! Role: selection.
//! Position: `frame/upload` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::bindings;
use crate::frame::engine::RenderEngine;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;

use crate::world::scene::ANCHOR;
use wasm_bindgen::prelude::*;
use website_graphics_engine::draw::geometry::LineVertex;
use website_graphics_engine::draw::{lines, polygons};
use website_graphics_engine::frame::{DrawBatch, DrawPayload};

#[wasm_bindgen]
impl RenderEngine {
    /// Upload marquee.
    pub fn upload_marquee(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        visible: bool,
    ) {
        if !visible || min_x >= max_x || min_y >= max_y {
            self.remove_lane(LaneRole::Marquee);
            self.remove_lane(LaneRole::MarqueeOutline);
            return;
        }

        let c = [173.0 / 255.0, 198.0 / 255.0, 1.0, 40.0 / 255.0];
        let corners = [
            [min_x, min_y],
            [max_x, min_y],
            [max_x, max_y],
            [min_x, max_y],
        ];
        let mut verts = Vec::with_capacity(4);
        for p in corners {
            verts.push(LineVertex {
                pos: [(p[0] - ANCHOR[0]) as f32, (p[1] - ANCHOR[1]) as f32],
                color: c,
            });
        }
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        let mesh = polygons::upload_indexed_mesh(
            &self.device,
            "marquee-verts",
            "marquee-indices",
            &verts,
            &indices,
            1,
        );
        self.upsert_lane(
            LaneRole::Marquee,
            DrawBatch {
                lane: lane_id(LaneRole::Marquee),
                visible: true,
                pipeline: bindings::PIPE_POLYGON,
                payload: DrawPayload::Indexed(mesh),
            },
        );

        let oc = [173.0 / 255.0, 198.0 / 255.0, 1.0, 200.0 / 255.0];
        let ring = [
            [min_x, min_y],
            [max_x, min_y],
            [max_x, max_y],
            [min_x, max_y],
        ];
        let mut outline = Vec::with_capacity(8);
        for e in 0..4 {
            let a = ring[e];
            let b = ring[(e + 1) % 4];
            outline.push(LineVertex {
                pos: [(a[0] - ANCHOR[0]) as f32, (a[1] - ANCHOR[1]) as f32],
                color: oc,
            });
            outline.push(LineVertex {
                pos: [(b[0] - ANCHOR[0]) as f32, (b[1] - ANCHOR[1]) as f32],
                color: oc,
            });
        }
        let stream = lines::upload_line_stream(&self.device, "marquee-outline", &outline);
        self.upsert_lane(
            LaneRole::MarqueeOutline,
            DrawBatch {
                lane: lane_id(LaneRole::MarqueeOutline),
                visible: true,
                pipeline: bindings::PIPE_LINE,
                payload: DrawPayload::Lines(stream),
            },
        );
    }
}
