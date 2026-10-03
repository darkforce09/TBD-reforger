//! **Role:** the slot drag: classify each drag update, upload the overlay of the dragged rows and
//! hide them in the base lane, move the overlay with the drag uniform, and restore on release.
//! **Position:** `symbology_layers_gpu::slot_symbology`, methods of [`SlotSymbology`]; the
//! Mission Creator's select tool and document host call [`SlotSymbology::set_drag`].
//! **Signals & state:** the slot bridge's drag ids and flag, the drag lane, the slot atlas's drag
//! uniform block.
//! **Invariants:** a drag delta writes 16 bytes of uniform and never repacks a lane; ending a drag
//! drops the drag lane, zeroes the offset (keeping the scale) and repacks the base lane.

use super::view::SlotSymbology;
use crate::icon_uniforms::ICON_DRAG_OFF;
use map_draw_lanes::lane_roles::LaneRole;
use overlay_instances::drag::DragGpuPhase;
use overlay_instances::drag::classify_drag_transition;
use overlay_instances::drag::pack_drag_overlay;
use overlay_instances::patches::hide_slot_row_patch;
use overlay_instances::symbols::SLOT_ICON_STRIDE;

impl SlotSymbology<'_> {
    /// Set drag.
    pub fn set_drag(&mut self, ids: Vec<String>, dx: f32, dy: f32) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let had = !self.slot_bridge.drag_ids.is_empty();
        let has = !ids.is_empty();
        let ids_changed = self.slot_bridge.drag_ids != ids;

        let delta_changed = had && has && !ids_changed;
        let phase = classify_drag_transition(had, has, ids_changed, delta_changed);
        match phase {
            DragGpuPhase::Idle => {}
            DragGpuPhase::End => {
                self.slot_bridge.drag_ids.clear();
                self.clear_slot_drag_internal();
            }
            DragGpuPhase::Delta => {
                if self.slot_bridge.drag_active {
                    self.set_slot_drag_delta(dx, dy);
                }
            }
            DragGpuPhase::Start | DragGpuPhase::Restart => {
                self.slot_bridge.drag_ids = ids;
                self.slot_bridge.drag_active = true;
                self.start_slot_drag_overlay(dx, dy);
            }
        }
    }

    /// Start slot drag overlay.
    pub(super) fn start_slot_drag_overlay(&mut self, dx: f32, dy: f32) {
        let drag_ids = self.slot_bridge.drag_ids.clone();
        if drag_ids.is_empty() {
            self.clear_slot_drag_internal();
            return;
        }
        self.slot_bridge.drag_active = true;

        let (overlay, rows) = match self.slot_bridge.symbology_base {
            Some(base) => overlay_instances::drag::pack_drag_overlay_symbology(
                &drag_ids,
                &self.slot_bridge.last_ids,
                &self.slot_bridge.last_xy,
                &self.slot_bridge.last_roles,
                &self.slot_bridge.last_headings,
                self.slot_m_per_px(),
                base,
            ),
            None => pack_drag_overlay(
                &drag_ids,
                &self.slot_bridge.last_ids,
                &self.slot_bridge.last_xy,
            ),
        };
        let count = rows.len();
        self.upload_slot_drag_lane(&overlay, count > 0);

        if !self.slot_bridge.slots_lane_selection_only {
            let hide = hide_slot_row_patch();
            for row in rows {
                #[allow(clippy::cast_possible_truncation)]
                let off = (row * SLOT_ICON_STRIDE + 8) as u32;
                self.patch_slot_lane(off, &hide);
            }
        }
        self.set_slot_drag_delta(dx, dy);
    }

    /// Clear slot drag internal.
    pub(super) fn clear_slot_drag_internal(&mut self) {
        self.slot_bridge.drag_active = false;
        self.slot_bridge.drag_ids.clear();
        self.clear_slot_drag_lane();
        self.rematerialize_slot_lane();
    }

    /// Set slot drag delta.
    pub(super) fn set_slot_drag_delta(&mut self, dx: f32, dy: f32) {
        let Some(atlas) = self.slot_atlas.as_mut() else {
            return;
        };
        atlas.drag_delta = [dx, dy];
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&dx.to_le_bytes());
        bytes[4..8].copy_from_slice(&dy.to_le_bytes());
        bytes[8..12].copy_from_slice(&atlas.px_to_m.to_le_bytes());

        self.lanes.layer_context().queue().write_buffer(
            &atlas.drag_uniform_buf,
            ICON_DRAG_OFF as u64,
            &bytes,
        );

        *self.uniform_bytes_last_frame = 64 + 16;

        self.lanes.mark_damage();
    }

    /// Upload slot drag lane.
    pub(super) fn upload_slot_drag_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::SlotDrag, bytes, visible);
    }

    /// Clear slot drag lane.
    pub(super) fn clear_slot_drag_lane(&mut self) {
        self.remove_lane(LaneRole::SlotDrag);
        if let Some(atlas) = self.slot_atlas.as_mut() {
            atlas.drag_delta = [0.0, 0.0];
            let mut bytes = [0u8; 16];
            bytes[8..12].copy_from_slice(&atlas.px_to_m.to_le_bytes());
            self.lanes.layer_context().queue().write_buffer(
                &atlas.drag_uniform_buf,
                ICON_DRAG_OFF as u64,
                &bytes,
            );
        }
    }
}
