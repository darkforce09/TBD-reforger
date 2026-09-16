//! Role: bridge 3.
//! Position: `symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::renderers::batching::batch::Batch;
use crate::renderers::batching::batch::BatchPayload;

use crate::symbology::instances::lanes::ICON_DRAG_OFF;
use crate::symbology::instances::symbols::SLOT_ICON_STRIDE;
use crate::symbology::roles::classify::SIDE_BLUFOR_RGBA;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl RenderEngine {
    /// Patch slot lane.
    pub(crate) fn patch_slot_lane(&mut self, byte_offset: u32, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        debug_assert!(
            !byte_offset.is_multiple_of(20),
            "patch_slot_lane is sub-row only (offset {byte_offset} is a stride boundary); \
             use upload_slot_lane for full rows"
        );
        let Some(batch) = self.batches.iter().find(|b| b.role == LaneRole::Slots) else {
            return;
        };
        let BatchPayload::IconInstanced { instances, count } = &batch.payload else {
            return;
        };
        let end = byte_offset as u64 + bytes.len() as u64;
        if end > u64::from(*count) * 20 {
            return;
        }
        self.queue
            .write_buffer(instances, u64::from(byte_offset), bytes);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload slot drag lane.
    pub(crate) fn upload_slot_drag_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::SlotDrag, bytes, visible);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Clear slot drag lane.
    pub(crate) fn clear_slot_drag_lane(&mut self) {
        self.remove_lane(LaneRole::SlotDrag);
        if let Some(atlas) = self.slot_atlas.as_mut() {
            atlas.drag_delta = [0.0, 0.0];
            let mut bytes = [0u8; 16];
            bytes[8..12].copy_from_slice(&atlas.px_to_m.to_le_bytes());
            self.queue
                .write_buffer(&atlas.drag_uniform_buf, ICON_DRAG_OFF as u64, &bytes);
        }
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload cluster lane.
    pub(crate) fn upload_cluster_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::Clusters, bytes, visible);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Vehicles bind.
    pub fn vehicles_bind(&mut self, xy: &[f32]) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let bytes = crate::symbology::instances::symbols::pack_vehicle_instances(xy);
        let vis = !bytes.is_empty();
        if !vis {
            self.remove_lane(LaneRole::MissionVehicles);
            return;
        }
        self.upload_slot_role_lane(LaneRole::MissionVehicles, &bytes, true);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Falls back to `vehicles_bind`'s disc lane when the atlas carries no symbology cells. Empty `xy` clears the lane.
    pub fn vehicles_bind_symbology(
        &mut self,
        xy: &[f32],
        aliases: Vec<String>,
        side_tints_rgba: &[u8],
        headings_deg: &[f32],
    ) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        if xy.len() < 2 {
            self.remove_lane(LaneRole::MissionVehicles);
            return;
        }
        let Some(base) = self.slot_bridge.symbology_base else {
            self.vehicles_bind(xy);
            return;
        };
        let n = xy.len() / 2;
        let tints: Vec<[u8; 4]> = (0..n)
            .map(|i| {
                side_tints_rgba
                    .get(i * 4..i * 4 + 4)
                    .map_or(SIDE_BLUFOR_RGBA, |s| [s[0], s[1], s[2], s[3]])
            })
            .collect();
        let bytes = crate::symbology::instances::symbols::pack_vehicle_symbology(
            xy,
            &aliases,
            &tints,
            headings_deg,
            self.slot_m_per_px(),
            base,
        );
        self.upload_slot_role_lane(LaneRole::MissionVehicles, &bytes, true);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Markers bind.
    pub fn markers_bind(
        &mut self,
        xy: &[f32],
        side_tints_rgba: &[u8],
        icons: Vec<String>,
        captions: Vec<String>,
    ) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let n = xy.len() / 2;
        if n == 0 {
            self.remove_lane(LaneRole::MissionMarkers);
            return;
        }

        let mut icon_bytes = Vec::with_capacity(n * SLOT_ICON_STRIDE);
        for i in 0..n {
            let x = xy[i * 2];
            let y = xy[i * 2 + 1];
            let rgba = if side_tints_rgba.len() >= (i + 1) * 4 {
                [
                    side_tints_rgba[i * 4],
                    side_tints_rgba[i * 4 + 1],
                    side_tints_rgba[i * 4 + 2],
                    side_tints_rgba[i * 4 + 3],
                ]
            } else {
                crate::symbology::roles::classify::SIDE_BLUFOR_RGBA
            };
            let glyph = icons
                .get(i)
                .map_or(crate::symbology::markers::MarkerGlyph::Disc, |a| {
                    crate::symbology::markers::marker_glyph_for_alias(a)
                }) as u16;
            crate::symbology::instances::packing::pack_icon_instance(
                &mut icon_bytes,
                x,
                y,
                crate::symbology::instances::symbols::SLOT_RING_PX,
                glyph,
                crate::symbology::instances::packing::pack_rgba_u32(rgba),
            );
        }

        let mut caption_bytes =
            crate::symbology::markers::pack_marker_caption_bytes(xy, &captions, self.zoom());
        if !caption_bytes.is_empty() {
            let _ = self.ensure_text_atlas();
        }
        self.upload_marker_composite(&mut icon_bytes, &mut caption_bytes);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Comments used to draw `SLOT_GLYPH_RING` in `SLOT_SELECTED_RGBA` — the selected-slot glyph in the selection colour — so every comment looked like a permanently selected unit and a genuinely selected comment was indistinguishable from an idle one. They now draw the neutral speech bubble (`slots_gpu::COMMENT_NOTE_RGBA`), with the amber reserved for actual selection.
    pub fn comments_bind(&mut self, xy: &[f32]) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        self.slot_bridge.comment_xy = xy.to_vec();
        let n = xy.len() / 2;
        if n == 0 {
            self.slot_bridge.comment_ids.clear();
            self.remove_lane(LaneRole::MissionComments);
            return;
        }

        let selected: Vec<bool> = (0..n)
            .map(|i| {
                self.slot_bridge
                    .comment_ids
                    .get(i)
                    .is_some_and(|id| self.slot_bridge.selected_ids.contains(id))
            })
            .collect();
        let bytes = match self.slot_bridge.symbology_base {
            Some(base) => crate::symbology::instances::symbols::pack_comment_instances(
                xy,
                &selected,
                self.slot_m_per_px(),
                base,
            ),

            None => {
                let mut b = Vec::with_capacity(n * SLOT_ICON_STRIDE);
                for (i, sel) in selected.iter().enumerate() {
                    let tint = crate::symbology::instances::packing::pack_rgba_u32(if *sel {
                        crate::symbology::instances::symbols::SLOT_SELECTED_RGBA
                    } else {
                        crate::symbology::instances::symbols::COMMENT_NOTE_RGBA
                    });
                    crate::symbology::instances::packing::pack_icon_instance(
                        &mut b,
                        xy[i * 2],
                        xy[i * 2 + 1],
                        crate::symbology::instances::symbols::SLOT_RING_PX,
                        crate::symbology::instances::symbols::SLOT_GLYPH_RING,
                        tint,
                    );
                }
                b
            }
        };
        self.upload_slot_role_lane(LaneRole::MissionComments, &bytes, true);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// `ids[i]` names comment `i` in the same id space [`Self::set_selection`] receives. Without them (what [`Self::comments_bind`] passes) the bubble and its neutral colour still land, but no comment can ever be shown as selected — the engine has no way to know which one is. Ids shorter than `xy` mark only the rows they cover.
    pub fn comments_bind_ids(&mut self, xy: &[f32], ids: Vec<String>) {
        self.slot_bridge.comment_ids = ids;
        self.comments_bind(xy);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Clear slot lanes.
    pub(crate) fn clear_slot_lanes(&mut self) {
        for role in [
            LaneRole::Slots,
            LaneRole::SlotDrag,
            LaneRole::Clusters,
            LaneRole::MissionVehicles,
            LaneRole::MissionComments,
        ] {
            self.clear_cull_lane(role);
            self.remove_lane(role);
        }
        self.remove_lane(LaneRole::MissionMarkers);
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Is pooled icon role.
    pub(crate) fn is_pooled_icon_role(role: LaneRole) -> bool {
        matches!(
            role,
            LaneRole::Slots
                | LaneRole::SlotDrag
                | LaneRole::Clusters
                | LaneRole::SlotPlacePreview
                | LaneRole::MissionVehicles
                | LaneRole::MissionComments
        )
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upsert pooled icon lane.
    pub(crate) fn upsert_pooled_icon_lane(
        &mut self,
        role: LaneRole,
        buf: wgpu::Buffer,
        count: u32,
        visible: bool,
        buffer_changed: bool,
    ) {
        if !buffer_changed
            && let Some(batch) = self.batches.iter_mut().find(|b| b.role == role)
            && let BatchPayload::IconInstanced {
                instances,
                count: stored,
            } = &mut batch.payload
        {
            *instances = buf;
            *stored = count;
            batch.visible = visible;
            self.damage.mark();
            return;
        }
        self.upsert_lane(
            role,
            Batch {
                role,
                visible,
                payload: BatchPayload::IconInstanced {
                    instances: buf,
                    count,
                },
            },
        );
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload slot role lane.
    pub(crate) fn upload_slot_role_lane(&mut self, role: LaneRole, bytes: &[u8], visible: bool) {
        const STRIDE: usize = 20;
        if bytes.is_empty() {
            if !visible {
                self.clear_cull_lane(role);
                self.remove_lane(role);
            }
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(role);
            return;
        }
        #[allow(clippy::cast_possible_truncation)]
        let count = (bytes.len() / STRIDE) as u32;

        let (buf, buffer_changed) = self.lane_pool.write_gpu(
            &self.device,
            &self.queue,
            role as u32,
            bytes,
            Self::convert_icon_world_to_anchor,
        );
        if self.gpu_cull_enabled() {
            let packed = self.lane_pool.contents(role as u32).to_vec();
            if let Some(cull) = &mut self.icon_cull {
                cull.upload_lane(&self.device, &self.queue, role as u32, &packed);
            }
            let _ = (buf, buffer_changed, count);
            self.remove_lane(role);
            self.damage.mark();
            return;
        }
        self.upsert_pooled_icon_lane(role, buf, count, visible, buffer_changed);
    }
}
