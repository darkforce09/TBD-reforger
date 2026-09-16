//! Role: selection.
//! Position: `renderers/primitives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::renderers::batching::batch::Batch;
use crate::renderers::batching::batch::BatchPayload;

use crate::renderers::batching::scene::ANCHOR;
use crate::renderers::primitives::hairlines::LineLane;
use crate::renderers::primitives::vector_lines::PolyLane;
use wasm_bindgen::prelude::*;

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
            verts.push(crate::renderers::batching::lanes::LineVertex {
                pos: [(p[0] - ANCHOR[0]) as f32, (p[1] - ANCHOR[1]) as f32],
                color: c,
            });
        }
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        use wgpu::util::DeviceExt;
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        self.upsert_lane(
            LaneRole::Marquee,
            Batch {
                role: LaneRole::Marquee,
                visible: true,
                payload: BatchPayload::Polygon(PolyLane {
                    verts: vbuf,
                    indices: ibuf,
                    index_count: 6,
                    item_count: 1,
                }),
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
            outline.push(crate::renderers::batching::lanes::LineVertex {
                pos: [(a[0] - ANCHOR[0]) as f32, (a[1] - ANCHOR[1]) as f32],
                color: oc,
            });
            outline.push(crate::renderers::batching::lanes::LineVertex {
                pos: [(b[0] - ANCHOR[0]) as f32, (b[1] - ANCHOR[1]) as f32],
                color: oc,
            });
        }
        let obuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-outline"),
                contents: bytemuck::cast_slice(&outline),
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.upsert_lane(
            LaneRole::MarqueeOutline,
            Batch {
                role: LaneRole::MarqueeOutline,
                visible: true,
                payload: BatchPayload::Lines(LineLane {
                    verts: obuf,
                    count: 8,
                }),
            },
        );
    }
}
