//! Role: buffers.
//! Position: `world/environment/buildings` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::bindings;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;

use crate::renderers::batching::scene::ANCHOR;
use wasm_bindgen::prelude::*;
use website_graphics_engine::draw::geometry::LineVertex;
use website_graphics_engine::draw::{lines as line_buffers, polygons};
use website_graphics_engine::frame::{DrawBatch, DrawPayload, InstanceBuffer};

#[wasm_bindgen]
impl RenderEngine {
    /// Upload world buildings.
    pub fn upload_world_buildings(&mut self, fill: &[f32], chunk_count: u32, visible: bool) {
        self.building_uploads += 1;
        const STRIDE: usize = 10;

        if fill.is_empty() {
            if !visible {
                self.remove_lane(LaneRole::WorldBuildings);
                self.world_chunks_drawn = 0;
            }
            return;
        }
        if !fill.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldBuildings);
            self.world_chunks_drawn = chunk_count;
            return;
        }
        let mut instances = Vec::with_capacity(fill.len() / STRIDE);
        for c in fill.chunks_exact(STRIDE) {
            instances.push(crate::renderers::batching::scene::BuildingInstance {
                center: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                half: [c[2], c[3]],
                basis: [c[4], c[5]],
                color: [c[6], c[7], c[8], c[9]],
            });
        }
        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world-buildings"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = instances.len() as u32;
        self.world_chunks_drawn = chunk_count;
        self.upsert_lane(
            LaneRole::WorldBuildings,
            DrawBatch {
                lane: lane_id(LaneRole::WorldBuildings),
                visible,
                pipeline: bindings::PIPE_BUILDING,
                payload: DrawPayload::OrientedQuads(InstanceBuffer::whole(buf, 40, count)),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload world building outlines.
    pub fn upload_world_building_outlines(&mut self, lines: &[f32], visible: bool) {
        self.building_uploads += 1;
        const STRIDE: usize = 6;
        if lines.is_empty() || !lines.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldBuildingsOutline);
            return;
        }
        let stream =
            line_buffers::upload_hairlines(&self.device, "world-buildings-outline", ANCHOR, lines);
        self.upsert_lane(
            LaneRole::WorldBuildingsOutline,
            DrawBatch {
                lane: lane_id(LaneRole::WorldBuildingsOutline),
                visible,
                pipeline: bindings::PIPE_LINE,
                payload: DrawPayload::Lines(stream),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload world fence strips.
    pub fn upload_world_fence_strips(&mut self, packed: &[f32], item_count: u32, visible: bool) {
        self.strip_lane_uploads += 1;
        const STRIDE: usize = 6;
        if packed.is_empty() {
            if !visible {
                self.remove_lane(LaneRole::WorldFences);
            }
            return;
        }
        if !packed.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldFences);
            return;
        }
        let n_verts = packed.len() / STRIDE;
        let mut verts = Vec::with_capacity(n_verts);
        for c in packed.chunks_exact(STRIDE) {
            verts.push(LineVertex {
                pos: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                color: [c[2], c[3], c[4], c[5]],
            });
        }
        #[allow(clippy::cast_possible_truncation)]
        let indices: Vec<u32> = (0..n_verts as u32).collect();
        let mesh = polygons::upload_indexed_mesh(
            &self.device,
            "world-fences",
            "world-fences-indices",
            &verts,
            &indices,
            item_count,
        );
        self.upsert_lane(
            LaneRole::WorldFences,
            DrawBatch {
                lane: lane_id(LaneRole::WorldFences),
                visible,
                pipeline: bindings::PIPE_POLYGON,
                payload: DrawPayload::Indexed(mesh),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Drop both world-building lanes (gate closed below the building band, or terrain switch).
    pub fn clear_world_buildings(&mut self) {
        self.remove_lane(LaneRole::WorldBuildings);
        self.remove_lane(LaneRole::WorldBuildingsOutline);
        self.remove_lane(LaneRole::WorldFences);
        self.world_chunks_drawn = 0;
    }
}
