//! Role: packer.
//! Position: `streaming/buffers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::culling::lod::INSTANCE_BUDGET;
use crate::core::culling::lod::class_visible;
use crate::environment::buildings::footprint::building_visible;
use crate::environment::vegetation::canopy::exact_tree_count;
use crate::environment::vegetation::canopy::heatmap_trees;
use crate::environment::vegetation::canopy::visible_tree_count;
use crate::streaming::scheduler::chunk_math::Bbox;
use crate::streaming::scheduler::chunk_math::chunk_ids_for_rect;
use crate::streaming::scheduler::chunk_math::chunk_rect_for_bbox;
use crate::streaming::scheduler::state::WorldResidency;
use crate::streaming::scheduler::viewport::DRAW_CULL_MARGIN_M;
use crate::symbology::labels::glyph_math::landmark_glyph_icon_key;

/// Deinterleave.
pub(crate) fn deinterleave(positions: &[f32], count: u32) -> (Vec<f32>, Vec<f32>) {
    let n = count as usize;
    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    for i in 0..n {
        xs.push(positions[2 * i]);
        ys.push(positions[2 * i + 1]);
    }
    (xs, ys)
}

impl WorldResidency {
    /// Early landmark glyph active.
    pub(crate) fn early_landmark_glyph_active(
        &self,
        cls: &str,
        importance_zoom: Option<f64>,
    ) -> bool {
        let z = self.deck_zoom;
        if class_visible("buildingBadge", z) {
            return false;
        }
        let Some(iz) = importance_zoom else {
            return false;
        };
        if z < iz {
            return false;
        }
        landmark_glyph_icon_key(cls)
            .and_then(|k| self.icon_key_to_idx.get(k))
            .is_some()
    }
}

impl WorldResidency {
    /// Fill band changed.
    pub(crate) fn fill_band_changed(&self, prev: f64, next: f64) -> bool {
        if building_visible(prev) != building_visible(next) {
            return true;
        }
        if class_visible("buildingBadge", prev) != class_visible("buildingBadge", next) {
            return true;
        }
        self.min_importance_zoom
            .is_some_and(|m| (prev >= m) != (next >= m))
    }
}

impl WorldResidency {
    /// Importance activation index.
    pub(crate) fn importance_activation_index(&self, z: f64) -> usize {
        self.importance_breakpoints.partition_point(|&t| t <= z)
    }
}

impl WorldResidency {
    /// Floor zoom term.
    pub(crate) fn floor_zoom_term(z: f64, floor_zoom: f64) -> u64 {
        if z >= floor_zoom {
            u64::MAX
        } else {
            z.to_bits()
        }
    }
}

impl WorldResidency {
    /// Glyph base sig.
    pub(crate) fn glyph_base_sig(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let z = self.deck_zoom;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.draw_ids.hash(&mut h);
        self.content_epoch.hash(&mut h);
        self.toggle_trees.hash(&mut h);
        self.toggle_props.hash(&mut h);
        self.toggle_buildings.hash(&mut h);
        self.toggle_airfield.hash(&mut h);
        self.airfield_bbox.map(|b| b.map(f64::to_bits)).hash(&mut h);

        class_visible("tree", z).hash(&mut h);
        class_visible("vegetation", z).hash(&mut h);
        class_visible("prop", z).hash(&mut h);
        class_visible("rockLarge", z).hash(&mut h);
        class_visible("buildingBadge", z).hash(&mut h);
        self.min_importance_zoom.map(f64::to_bits).hash(&mut h);
        self.importance_activation_index(z).hash(&mut h);
        Self::floor_zoom_term(z, self.glyph_size_floor_zoom).hash(&mut h);
        h.finish()
    }
}

impl WorldResidency {
    /// Draw chunk ids.
    #[must_use]
    pub fn draw_chunk_ids(&self, strict_bbox: Bbox) -> Vec<String> {
        debug_assert_eq!(DRAW_CULL_MARGIN_M, 0.0);
        let chunk_size_m = match &self.manifest {
            Some(m) => m.chunk_size_m,
            None => return Vec::new(),
        };
        let rect = chunk_rect_for_bbox(strict_bbox, self.terrain, chunk_size_m);
        let mut ids = chunk_ids_for_rect(rect);
        if let Some(cells) = &self.cell_ids {
            ids.retain(|id| cells.contains(id));
        }
        ids.retain(|id| self.pinned_set.contains(id));
        ids.sort();
        ids
    }
}

impl WorldResidency {
    /// Refresh draw set and glyphs.
    pub(crate) fn refresh_draw_set_and_glyphs(&mut self) {
        self.draw_ids = self.draw_chunk_ids(self.last_viewport);
        let chunk_size_m = self
            .manifest
            .as_ref()
            .map(|m| m.chunk_size_m)
            .unwrap_or(512.0);
        let mut changed = false;

        let glyph_key = self.glyph_base_sig();
        if glyph_key != self.glyph_base_key {
            self.glyph_base_key = glyph_key;

            let visible = visible_tree_count(
                &self.chunks,
                &self.draw_ids,
                self.last_viewport,
                chunk_size_m,
            );
            let reenter = INSTANCE_BUDGET * 85 / 100;
            self.heatmap_trees = if self.heatmap_trees {
                visible >= reenter
            } else {
                heatmap_trees(visible)
            };
            self.exact_tree_count =
                exact_tree_count(&self.chunks, &self.draw_ids, self.deck_zoom) as u32;
            self.rebuild_glyph_buffers();
            self.glyph_recomposes += 1;
            changed = true;
        }

        let strip_key = self.strip_compose_key();
        if strip_key != self.strip_key {
            self.strip_key = strip_key;
            self.rebuild_strip_buffers();
            changed = true;
        }

        if changed {
            self.buffers_revision += 1;
        }
    }
}
