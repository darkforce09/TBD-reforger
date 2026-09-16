//! Role: buffers.
//! Position: `environment/buildings` in the graphics engine.
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
            Batch {
                role: LaneRole::WorldBuildings,
                visible,
                payload: BatchPayload::BuildingInstanced {
                    instances: buf,
                    count,
                },
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
        let mut verts = Vec::with_capacity(lines.len() / STRIDE);
        for c in lines.chunks_exact(STRIDE) {
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
                label: Some("world-buildings-outline"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = verts.len() as u32;
        self.upsert_lane(
            LaneRole::WorldBuildingsOutline,
            Batch {
                role: LaneRole::WorldBuildingsOutline,
                visible,
                payload: BatchPayload::Lines(LineLane { verts: buf, count }),
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
                label: Some("world-fences"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world-fences-indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let index_count = indices.len() as u32;
        self.upsert_lane(
            LaneRole::WorldFences,
            Batch {
                role: LaneRole::WorldFences,
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
    /// Drop both world-building lanes (gate closed below the building band, or terrain switch).
    pub fn clear_world_buildings(&mut self) {
        self.remove_lane(LaneRole::WorldBuildings);
        self.remove_lane(LaneRole::WorldBuildingsOutline);
        self.remove_lane(LaneRole::WorldFences);
        self.world_chunks_drawn = 0;
    }
}
