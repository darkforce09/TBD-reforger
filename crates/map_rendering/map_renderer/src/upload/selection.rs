//! **Role:** the selection marquee: a translucent fill and its outline.
//! **Position:** the map renderer's upload belts; the Mission Creator's select tool uploads it
//! while a marquee drag is live.
//! **Signals & state:** the `Marquee` and `MarqueeOutline` lanes.
//! **Invariants:** a hidden or empty marquee removes both lanes; the corners cross to the GPU
//! relative to the scene anchor.

use crate::engine::RenderEngine;
use gpu_frame::draw::{lines, polygons};
use gpu_frame::frame::{DrawBatch, DrawPayload};
use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use render_primitives::draw::geometry::LineVertex;
use renderer_core::packet_bindings;

impl RenderEngine {
    /// Upload the marquee over the world rectangle `[min_x, min_y]…[max_x, max_y]`, or remove it
    /// when hidden or empty.
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
            self.gpu.device(),
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
                pipeline: packet_bindings::PIPE_POLYGON,
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
        let stream = lines::upload_line_stream(self.gpu.device(), "marquee-outline", &outline);
        self.upsert_lane(
            LaneRole::MarqueeOutline,
            DrawBatch {
                lane: lane_id(LaneRole::MarqueeOutline),
                visible: true,
                pipeline: packet_bindings::PIPE_LINE,
                payload: DrawPayload::Lines(stream),
            },
        );
    }
}
