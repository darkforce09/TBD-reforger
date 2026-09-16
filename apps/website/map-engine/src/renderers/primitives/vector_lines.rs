//! Role: vector lines.
//! Position: `renderers/primitives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::bindings;
use crate::core::pipeline::draw_order::lane_id;
use crate::core::pipeline::draw_order::lane_role_from_u32;

use crate::renderers::batching::scene::ANCHOR;
use wasm_bindgen::prelude::*;
use website_graphics_engine::draw::polygons;
use website_graphics_engine::frame::{DrawBatch, DrawPayload};

// T-0xx Phase 1D: `PolyLane` is gone — it is `website_graphics_engine::frame::IndexedMesh`,
// and the two loops that filled it are `draw::polygons::{upload_polygon_mesh, upload_strip_mesh}`.

#[wasm_bindgen]
impl RenderEngine {
    /// Upload polygon mesh.
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
            &self.device,
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
                pipeline: bindings::PIPE_POLYGON,
                payload: DrawPayload::Indexed(mesh),
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload strip tris.
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
        let mesh = polygons::upload_strip_mesh(&self.device, ANCHOR, packed, item_count);
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            DrawBatch {
                lane: lane_id(role_enum),
                visible,
                pipeline: bindings::PIPE_POLYGON,
                payload: DrawPayload::Indexed(mesh),
            },
        );
    }
}
