//! The draw set refresh, the glyph memo key and the fill-band test.
//!
//! **Role:** refreshes the draw set from the residency's pin and viewport, decides the tree
//! heatmap rung, and recomposes the glyph and strip buffers only when their memo keys change;
//! answers whether a zoom move crosses a building fill band.
//! **Position:** `chunk_draw_buffers`; methods of
//! `DrawBuffers` (`draw_buffers.rs`) reading a
//! [`chunk_scheduler::state::ChunkResidency`]; called by the world residency's
//! rebuild requests, the footprint rebuild and the toggles.
//! **Signals & state:** writes the draw set, the glyph memo key, the heatmap state, the exact tree
//! count, the glyph and strip buffers and `buffers_revision`.
//! **Invariants:** an unchanged memo key recomposes nothing; the heatmap enters above
//! `INSTANCE_BUDGET` visible trees and leaves below 85 % of it.

use crate::draw_buffers::DrawBuffers;
use chunk_scheduler::state::ChunkResidency;
use label_layout::glyph_math::landmark_glyph_icon_key;
use map_draw_lanes::zoom_gates::INSTANCE_BUDGET;
use map_draw_lanes::zoom_gates::building_visible;
use map_draw_lanes::zoom_gates::class_visible;
use vegetation::canopy::exact_tree_count;
use vegetation::canopy::heatmap_trees;
use vegetation::canopy::visible_tree_count;

impl DrawBuffers {
    /// Early landmark glyph active.
    pub(super) fn early_landmark_glyph_active(
        &self,
        deck_zoom: f64,
        cls: &str,
        importance_zoom: Option<f64>,
    ) -> bool {
        let z = deck_zoom;
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

impl DrawBuffers {
    /// Whether a zoom move from `prev` to `next` crosses the building gate, the badge band or the lowest importance zoom.
    pub(super) fn fill_band_changed(
        min_importance_zoom: Option<f64>,
        prev: f64,
        next: f64,
    ) -> bool {
        if building_visible(prev) != building_visible(next) {
            return true;
        }
        if class_visible("buildingBadge", prev) != class_visible("buildingBadge", next) {
            return true;
        }
        min_importance_zoom.is_some_and(|m| (prev >= m) != (next >= m))
    }
}

impl DrawBuffers {
    /// Importance activation index.
    pub(super) fn importance_activation_index(importance_breakpoints: &[f64], z: f64) -> usize {
        importance_breakpoints.partition_point(|&t| t <= z)
    }
}

impl DrawBuffers {
    /// Floor zoom term.
    pub(super) fn floor_zoom_term(z: f64, floor_zoom: f64) -> u64 {
        if z >= floor_zoom {
            u64::MAX
        } else {
            z.to_bits()
        }
    }
}

impl DrawBuffers {
    /// Glyph base sig.
    pub(super) fn glyph_base_sig(&self, residency: &ChunkResidency) -> u64 {
        use std::hash::{Hash, Hasher};
        let z = residency.deck_zoom();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.draw_ids.hash(&mut h);
        residency.content_epoch().hash(&mut h);
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
        residency
            .min_importance_zoom()
            .map(f64::to_bits)
            .hash(&mut h);
        Self::importance_activation_index(residency.importance_breakpoints(), z).hash(&mut h);
        Self::floor_zoom_term(z, self.glyph_size_floor_zoom).hash(&mut h);
        h.finish()
    }
}

impl DrawBuffers {
    /// Refresh draw set and glyphs.
    pub(super) fn refresh_draw_set_and_glyphs(&mut self, residency: &ChunkResidency) {
        self.draw_ids = residency.draw_chunk_ids(residency.last_viewport());
        let chunk_size_m = residency.chunk_size_m();
        let mut changed = false;

        let glyph_key = self.glyph_base_sig(residency);
        if glyph_key != self.glyph_base_key {
            self.glyph_base_key = glyph_key;

            let visible = visible_tree_count(
                residency.resident_chunks(),
                &self.draw_ids,
                residency.last_viewport(),
                chunk_size_m,
            );
            let reenter = INSTANCE_BUDGET * 85 / 100;
            self.heatmap_trees = if self.heatmap_trees {
                visible >= reenter
            } else {
                heatmap_trees(visible)
            };
            self.exact_tree_count = exact_tree_count(
                residency.resident_chunks(),
                &self.draw_ids,
                residency.deck_zoom(),
            ) as u32;
            self.rebuild_glyph_buffers(residency);
            self.glyph_recomposes += 1;
            changed = true;
        }

        let strip_key = self.strip_compose_key(residency);
        if strip_key != self.strip_key {
            self.strip_key = strip_key;
            self.rebuild_strip_buffers(residency);
            changed = true;
        }

        if changed {
            self.buffers_revision += 1;
        }
    }
}
