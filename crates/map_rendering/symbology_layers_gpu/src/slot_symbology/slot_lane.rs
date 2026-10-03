//! **Role:** the slot lane: bind the slot columns, repack the lane as symbology, discs or the
//! selection-only cluster view, and patch the rows a selection change flips.
//! **Position:** `symbology_layers_gpu::slot_symbology`, methods of [`SlotSymbology`]; the
//! Mission Creator's document host and entity selection call [`SlotSymbology::slots_bind_symbology`]
//! and [`SlotSymbology::set_selection`].
//! **Signals & state:** the slot bridge's columns and selection, and the slot lane's pooled buffer.
//! **Invariants:** a selection change patches only the flipped rows, 12 bytes each at instance
//! offset 8, and never repacks the lane, except while the lane holds only the selection (cluster
//! mode) or a drag is live; every selection change refreshes the comment lane.

use super::view::SlotSymbology;
use gpu_frame::frame::DrawPayload;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use overlay_instances::patches::pack_selection_only;
use overlay_instances::patches::selected_mask;
use overlay_instances::patches::selected_row_patch;
use overlay_instances::patches::unselected_row_patch_for;
use overlay_instances::symbols::SLOT_ICON_STRIDE;
use overlay_instances::symbols::pack_slot_instances;
use unit_symbology::classification::SIDE_BLUFOR_RGBA;

impl SlotSymbology<'_> {
    /// Both are tolerated short row-by-row rather than required, so the feeder can be wired one column at a time without a bind that panics or silently drops rows.
    pub fn slots_bind_symbology(
        &mut self,
        ids: Vec<String>,
        xy: &[f32],
        side_tints_rgba: &[u8],
        roles: Vec<String>,
        headings_deg: &[f32],
    ) {
        self.slot_bridge.last_roles = roles;
        self.slot_bridge.last_headings = headings_deg.to_vec();
        let n = ids.len();
        let mut tints = Vec::with_capacity(n);
        for i in 0..n {
            let o = i * 4;
            if o + 4 <= side_tints_rgba.len() {
                tints.push([
                    side_tints_rgba[o],
                    side_tints_rgba[o + 1],
                    side_tints_rgba[o + 2],
                    side_tints_rgba[o + 3],
                ]);
            } else {
                tints.push(SIDE_BLUFOR_RGBA);
            }
        }
        self.slot_bridge.last_ids = ids;
        self.slot_bridge.last_xy = xy.to_vec();
        self.slot_bridge.last_side_tints = tints;

        self.slot_bridge.cluster_index = None;
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        self.slot_bridge.last_cluster_mode = overlay_instances::symbols::cluster_mode(n, zoom);
        if self.slot_bridge.drag_active && !self.slot_bridge.drag_ids.is_empty() {
            let dx = self
                .slot_atlas
                .as_ref()
                .map(|a| a.drag_delta[0])
                .unwrap_or(0.0);
            let dy = self
                .slot_atlas
                .as_ref()
                .map(|a| a.drag_delta[1])
                .unwrap_or(0.0);
            self.start_slot_drag_overlay(dx, dy);
        } else {
            self.rematerialize_slot_lane();
        }

        self.feed_cluster_markers();
    }

    /// Rematerialize slot lane.
    pub(super) fn rematerialize_slot_lane(&mut self) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let mask = selected_mask(&self.slot_bridge.last_ids, &self.slot_bridge.selected_ids);
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        let cm = overlay_instances::symbols::cluster_mode(n, zoom);
        self.slot_bridge.last_cluster_mode = cm;
        if cm {
            let bytes = pack_selection_only(&self.slot_bridge.last_xy, &mask);
            let vis = !bytes.is_empty();
            self.upload_slot_lane(&bytes, vis);
            self.slot_bridge.slots_lane_selection_only = true;
        } else {
            let bytes = match self.slot_bridge.symbology_base {
                Some(base) => overlay_instances::symbols::pack_slot_symbology(
                    &self.slot_bridge.last_xy,
                    &mask,
                    &self.slot_bridge.last_side_tints,
                    &self.slot_bridge.last_roles,
                    &self.slot_bridge.last_headings,
                    self.slot_m_per_px(),
                    base,
                ),
                None => pack_slot_instances(
                    &self.slot_bridge.last_xy,
                    &mask,
                    &self.slot_bridge.last_side_tints,
                ),
            };
            let vis = !self.slot_bridge.last_ids.is_empty();
            self.upload_slot_lane(&bytes, vis);
            self.slot_bridge.slots_lane_selection_only = false;
        }
        self.slot_bridge.last_symbology_detailed = self.symbology_detailed();
    }

    /// Set selection.
    pub fn set_selection(&mut self, ids: Vec<String>) {
        let new_sel: std::collections::HashSet<String> = ids.into_iter().collect();
        if !self.slot_bridge.atlas_ready {
            self.slot_bridge.selected_ids = new_sel;
            return;
        }

        if self.slot_bridge.drag_active && self.slot_bridge.drag_ids.is_empty() {
            self.slot_bridge.selected_ids = new_sel;
            self.clear_slot_drag_internal();
            self.refresh_comment_lane();
            return;
        }
        if self.slot_bridge.drag_active {
            self.slot_bridge.selected_ids = new_sel;
            self.refresh_comment_lane();
            return;
        }

        if self.slot_bridge.slots_lane_selection_only {
            self.slot_bridge.selected_ids = new_sel;
            self.rematerialize_slot_lane();
            self.refresh_comment_lane();
            return;
        }

        let old_sel = std::mem::replace(&mut self.slot_bridge.selected_ids, new_sel);
        let flips: Vec<(usize, bool)> = {
            let sb = &self.slot_bridge;
            sb.last_ids
                .iter()
                .enumerate()
                .filter_map(|(row, id)| {
                    let now = sb.selected_ids.contains(id);
                    (old_sel.contains(id) != now).then_some((row, now))
                })
                .collect()
        };
        if flips.is_empty() {
            self.refresh_comment_lane();
            return;
        }
        let m_per_px = self.slot_m_per_px();
        for (row, now) in &flips {
            let rgba = self
                .slot_bridge
                .last_side_tints
                .get(*row)
                .copied()
                .unwrap_or(SIDE_BLUFOR_RGBA);

            let patch = match self.slot_bridge.symbology_base {
                Some(base) => overlay_instances::patches::symbology_row_patch(
                    *now,
                    self.slot_bridge
                        .last_roles
                        .get(*row)
                        .map_or("", String::as_str),
                    self.slot_bridge
                        .last_headings
                        .get(*row)
                        .copied()
                        .unwrap_or(0.0),
                    rgba,
                    m_per_px,
                    base,
                ),
                None if *now => selected_row_patch(),
                None => unselected_row_patch_for(rgba),
            };
            #[allow(clippy::cast_possible_truncation)]
            let off = (row * SLOT_ICON_STRIDE + 8) as u32;
            self.patch_slot_lane(off, &patch);
        }
        self.lanes.mark_damage();

        self.refresh_comment_lane();
    }

    /// Patch slot lane.
    pub(super) fn patch_slot_lane(&mut self, byte_offset: u32, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        debug_assert!(
            !byte_offset.is_multiple_of(20),
            "patch_slot_lane is sub-row only (offset {byte_offset} is a stride boundary); \
             use upload_slot_lane for full rows"
        );
        let slots = lane_id(LaneRole::Slots);
        let Some(batch) = self.lanes.lane_batch(slots) else {
            return;
        };
        let DrawPayload::Sprites { instances, .. } = &batch.payload else {
            return;
        };
        let end = byte_offset as u64 + bytes.len() as u64;
        if end > u64::from(instances.count) * 20 {
            return;
        }
        let buffer = instances.buffer.clone();
        self.lanes
            .layer_context()
            .queue()
            .write_buffer(&buffer, u64::from(byte_offset), bytes);
    }

    /// Upload slot lane.
    pub(super) fn upload_slot_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::Slots, bytes, visible);
    }
}
