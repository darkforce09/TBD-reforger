//! Role: lanes.
//! Position: `overlay/symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::bindings;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;

use crate::renderers::batching::scene::ANCHOR;
use wasm_bindgen::prelude::*;
use website_graphics_engine::frame::{DrawBatch, DrawPayload, InstanceBuffer};

/// Canonical icon uv bytes value.
pub(crate) const ICON_UV_BYTES: usize = crate::renderers::batching::scene::ATLAS_GLYPH_COUNT * 16;

/// Canonical icon uniform bytes value.
pub(crate) const ICON_UNIFORM_BYTES: u64 = (ICON_UV_BYTES + 16) as u64;

/// Canonical icon drag off value.
pub(crate) const ICON_DRAG_OFF: usize = ICON_UV_BYTES;

/// Canonical icon pxm off value.
pub(crate) const ICON_PXM_OFF: usize = ICON_UV_BYTES + 8;

#[wasm_bindgen]
impl RenderEngine {
    /// Upload icon lane.
    pub fn upload_icon_lane(&mut self, kind: u32, bytes: &[u8], visible: bool) {
        self.icon_lane_uploads += 1;
        let role = match kind {
            0 => LaneRole::WorldTrees,
            1 => LaneRole::WorldProps,
            2 => LaneRole::WorldBadges,
            _ => return,
        };
        const STRIDE: usize = 20;
        if bytes.is_empty() {
            if role == LaneRole::WorldTrees {
                self.tree_icons_20.clear();
            }
            self.clear_cull_lane(role);

            self.remove_lane(role);
            self.damage.mark();
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(role);
            return;
        }

        let mut converted = bytes.to_vec();
        for chunk in converted.chunks_exact_mut(STRIDE) {
            let x = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let y = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            let ax = (f64::from(x) - ANCHOR[0]) as f32;
            let ay = (f64::from(y) - ANCHOR[1]) as f32;
            chunk[0..4].copy_from_slice(&ax.to_le_bytes());
            chunk[4..8].copy_from_slice(&ay.to_le_bytes());
        }

        if self.gpu_cull_enabled() {
            if role == LaneRole::WorldTrees {
                self.tree_icons_20 = converted.clone();
            }
            if let Some(cull) = &mut self.icon_cull {
                cull.upload_lane(&self.device, &self.queue, role as u32, &converted);
            }
            self.remove_lane(role);
            self.damage.mark();
            return;
        }

        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("icon-lane"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = (converted.len() / STRIDE) as u32;
        self.upsert_lane(
            role,
            DrawBatch {
                lane: lane_id(role),
                visible,
                pipeline: bindings::PIPE_ICON,
                #[allow(clippy::cast_possible_truncation)]
                payload: DrawPayload::Sprites {
                    instances: InstanceBuffer::whole(buf, STRIDE as u32, count),
                    atlas: bindings::sprite_atlas_for(role),
                },
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Drop all three glyph icon lanes.
    pub fn clear_icon_lanes(&mut self) {
        self.tree_icons_20.clear();
        for role in [
            LaneRole::WorldTrees,
            LaneRole::WorldProps,
            LaneRole::WorldBadges,
        ] {
            self.clear_cull_lane(role);
            self.remove_lane(role);
        }
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Pack icon uniforms.
    pub(crate) fn pack_icon_uniforms(
        uv: &[f32],
        drag_dx: f32,
        drag_dy: f32,
        px_to_m: f32,
    ) -> Vec<u8> {
        let mut u_bytes = vec![0u8; ICON_UNIFORM_BYTES as usize];
        for (i, v) in uv.iter().enumerate() {
            let off = i * 4;
            if off + 4 <= ICON_UV_BYTES {
                u_bytes[off..off + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        u_bytes[ICON_DRAG_OFF..ICON_DRAG_OFF + 4].copy_from_slice(&drag_dx.to_le_bytes());
        u_bytes[ICON_DRAG_OFF + 4..ICON_DRAG_OFF + 8].copy_from_slice(&drag_dy.to_le_bytes());
        u_bytes[ICON_PXM_OFF..ICON_PXM_OFF + 4].copy_from_slice(&px_to_m.to_le_bytes());
        u_bytes
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Convert icon world to anchor.
    pub(crate) fn convert_icon_world_to_anchor(bytes: &mut [u8]) {
        const STRIDE: usize = 20;
        for chunk in bytes.chunks_exact_mut(STRIDE) {
            let x = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let y = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            let ax = (f64::from(x) - ANCHOR[0]) as f32;
            let ay = (f64::from(y) - ANCHOR[1]) as f32;
            chunk[0..4].copy_from_slice(&ax.to_le_bytes());
            chunk[4..8].copy_from_slice(&ay.to_le_bytes());
        }
    }
}
