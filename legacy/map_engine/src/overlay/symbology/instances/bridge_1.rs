//! Role: bridge 1.
//! Position: `overlay/symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::engine::RenderEngine;
use crate::overlay::symbology::instances::drag::DragGpuPhase;
use crate::overlay::symbology::instances::drag::classify_drag_transition;
use crate::overlay::symbology::instances::patches::selected_row_patch;
use crate::overlay::symbology::instances::patches::unselected_row_patch_for;
use crate::overlay::symbology::instances::symbols::SLOT_ICON_STRIDE;
use crate::overlay::symbology::instances::symbols::pack_cluster_instances;
use crate::overlay::symbology::roles::classify::SIDE_BLUFOR_RGBA;
use crate::world::scene::EVERON_BOUNDS;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;

/// Slot gpu bridge.
#[derive(Default)]
pub(crate) struct SlotGpuBridge {
    /// Atlas ready.
    pub(crate) atlas_ready: bool,

    /// Last ids.
    pub(crate) last_ids: Vec<String>,

    /// Last xy.
    pub(crate) last_xy: Vec<f32>,

    /// Last side tints.
    pub(crate) last_side_tints: Vec<[u8; 4]>,

    /// Selected ids.
    pub(crate) selected_ids: HashSet<String>,

    /// Last cluster mode.
    pub(crate) last_cluster_mode: bool,

    /// Drag active.
    pub(crate) drag_active: bool,

    /// Drag ids.
    pub(crate) drag_ids: Vec<String>,

    /// Slots lane selection only.
    pub(crate) slots_lane_selection_only: bool,

    /// Cluster index.
    pub(crate) cluster_index: Option<crate::spatial::indexing::cluster::ClusterIndex>,

    /// Cluster built len.
    pub(crate) cluster_built_len: usize,

    /// Symbology base.
    pub(crate) symbology_base: Option<u16>,

    /// Last roles.
    pub(crate) last_roles: Vec<String>,

    /// Last headings.
    pub(crate) last_headings: Vec<f32>,

    /// Comment xy.
    pub(crate) comment_xy: Vec<f32>,

    /// Comment ids.
    pub(crate) comment_ids: Vec<String>,

    /// Last symbology detailed.
    pub(crate) last_symbology_detailed: bool,
}

/// Slot atlas gpu.
pub(crate) struct SlotAtlasGpu {
    /// Texture.
    pub(crate) texture: wgpu::Texture,

    /// Base uniform buf.
    pub(crate) base_uniform_buf: wgpu::Buffer,

    /// Drag uniform buf.
    pub(crate) drag_uniform_buf: wgpu::Buffer,

    /// Base bind group.
    pub(crate) base_bind_group: wgpu::BindGroup,

    /// Drag bind group.
    pub(crate) drag_bind_group: wgpu::BindGroup,

    /// Bytes.
    pub(crate) bytes: u64,

    /// Px to m.
    pub(crate) px_to_m: f32,

    /// Drag delta.
    pub(crate) drag_delta: [f32; 2],
}

#[wasm_bindgen]
impl RenderEngine {
    /// Upload dedicated slot/cluster atlas once. Replaces low-level `upload_slot_atlas` as the TS entry point.
    pub fn ensure_slot_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<(), JsError> {
        match crate::overlay::symbology::atlas::raster::extend_atlas_with_unit_glyphs(
            rgba, width, height,
        ) {
            Some(wide) => {
                self.upload_slot_atlas(&wide.rgba, wide.width, wide.height, &wide.uv)?;
                self.slot_bridge.symbology_base = Some(wide.base_cells);
            }
            None => {
                self.upload_slot_atlas(rgba, width, height, uv)?;
                self.slot_bridge.symbology_base = None;
            }
        }
        self.slot_bridge.atlas_ready = true;
        self.slot_bridge.last_symbology_detailed = self.symbology_detailed();
        self.sync_slot_zoom_uniform();
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Slot m per px.
    pub(crate) fn slot_m_per_px(&self) -> f32 {
        crate::overlay::symbology::instances::symbols::px_to_m_at_zoom(self.zoom())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Symbology detailed.
    pub(crate) fn symbology_detailed(&self) -> bool {
        crate::overlay::symbology::instances::symbols::symbology_visible(self.slot_m_per_px())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Slots bind soa.
    pub fn slots_bind_soa(&mut self, ids: Vec<String>, xy: &[f32], side_tints_rgba: &[u8]) {
        self.slots_bind_symbology(ids, xy, side_tints_rgba, Vec::new(), &[]);
    }
}

#[wasm_bindgen]
impl RenderEngine {
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
        self.slot_bridge.last_cluster_mode =
            crate::overlay::symbology::instances::symbols::cluster_mode(n, zoom);
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
}

#[wasm_bindgen]
impl RenderEngine {
    /// Feed cluster markers.
    pub(crate) fn feed_cluster_markers(&mut self) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let n = self.slot_bridge.last_ids.len();
        #[allow(clippy::cast_possible_truncation)]
        if !crate::overlay::symbology::instances::symbols::cluster_mode(n as u32, self.zoom()) {
            self.upload_cluster_lane(&[], false);
            return;
        }

        if self.slot_bridge.cluster_index.is_none() || self.slot_bridge.cluster_built_len != n {
            let world: Vec<(f64, f64)> = self
                .slot_bridge
                .last_xy
                .chunks_exact(2)
                .map(|c| (f64::from(c[0]), f64::from(c[1])))
                .collect();
            self.slot_bridge.cluster_index =
                Some(crate::spatial::indexing::cluster::ClusterIndex::build(
                    &world,
                    EVERON_BOUNDS[2],
                    EVERON_BOUNDS[3],
                ));
            self.slot_bridge.cluster_built_len = n;
        }
        let rect = self.camera.visible_world_rect();
        let zoom = self.zoom();
        let (xs, ys, counts) = {
            let idx = self
                .slot_bridge
                .cluster_index
                .as_ref()
                .expect("built above");
            let markers = idx.get_clusters(rect[0], rect[1], rect[2], rect[3], zoom);
            let xs: Vec<f64> = markers.iter().map(|m| m.x).collect();
            let ys: Vec<f64> = markers.iter().map(|m| m.y).collect();
            let counts: Vec<u32> = markers.iter().map(|m| m.count).collect();
            (xs, ys, counts)
        };
        let bytes = pack_cluster_instances(&xs, &ys, &counts);
        self.upload_cluster_lane(&bytes, !bytes.is_empty());
    }
}

#[wasm_bindgen]
impl RenderEngine {
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
                Some(base) => crate::overlay::symbology::instances::patches::symbology_row_patch(
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
        self.damage.mark();

        self.refresh_comment_lane();
    }
}

#[wasm_bindgen]
impl RenderEngine {
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
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set cluster markers.
    pub fn set_cluster_markers(&mut self, xs: &[f64], ys: &[f64], counts: &[u32]) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        let cm = crate::overlay::symbology::instances::symbols::cluster_mode(n, zoom);
        self.slot_bridge.last_cluster_mode = cm;
        if !cm {
            self.upload_cluster_lane(&[], false);
            if !self.slot_bridge.drag_active {
                self.rematerialize_slot_lane();
            }
            return;
        }
        let bytes = pack_cluster_instances(xs, ys, counts);
        self.upload_cluster_lane(&bytes, !bytes.is_empty());
        if !self.slot_bridge.drag_active {
            self.rematerialize_slot_lane();
        }
    }
}
