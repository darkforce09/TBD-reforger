//! The draw buffer getters the world loader uploads from, and the buffers revision.
//!
//! **Role:** hands out the composed buffers (building fill and outline, strips, tree, prop and
//! badge glyphs), their counts, the lane-off flags, the draw set, the tree heatmap state and the
//! revision that tells the loader a buffer changed.
//! **Position:** `chunk_draw_buffers`; methods of
//! [`crate::world_residency::WorldResidency`]; read by the world loader's
//! upload and terrain passes and the residency tests.
//! **Signals & state:** none; every getter reads or clones.
//! **Invariants:** the buffers returned are the last composed ones; `buffers_revision` only grows.

use map_draw_lanes::zoom_gates::class_visible;

use crate::world_residency::WorldResidency;
use world_chunks::ChunkId;

impl WorldResidency {
    /// Building fill instances (WORLD coords): 10 f32 each `[x, y, hx, hy, cos, sin, r, g, b, a]`.
    #[must_use]
    pub fn world_building_fill(&self) -> Vec<f32> {
        self.draw_buffers.fill_buf.clone()
    }
}

impl WorldResidency {
    /// Building outline vertices (WORLD coords): 6 f32 each `[x, y, r, g, b, a]`, `LineList`.
    #[must_use]
    pub fn world_building_outline(&self) -> Vec<f32> {
        self.draw_buffers.outline_buf.clone()
    }
}

impl WorldResidency {
    /// World fence strips.
    #[must_use]
    pub fn world_fence_strips(&self) -> Vec<f32> {
        self.draw_buffers.strip_buf.clone()
    }
}

impl WorldResidency {
    /// Fence strip segment count.
    #[must_use]
    pub fn fence_strip_segment_count(&self) -> u32 {
        self.draw_buffers.fence_strip_count
    }
}

impl WorldResidency {
    /// Pier strip segment count.
    #[must_use]
    pub fn pier_strip_segment_count(&self) -> u32 {
        self.draw_buffers.pier_strip_count
    }
}

impl WorldResidency {
    /// Bridge rail strip count.
    #[must_use]
    pub fn bridge_rail_strip_count(&self) -> u32 {
        self.draw_buffers.bridge_rail_count
    }
}

impl WorldResidency {
    /// Packed tree+vegetation icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_tree_glyphs(&self) -> Vec<u8> {
        self.draw_buffers.tree_glyph_buf.clone()
    }
}

impl WorldResidency {
    /// Packed prop+rockLarge icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_prop_glyphs(&self) -> Vec<u8> {
        self.draw_buffers.prop_glyph_buf.clone()
    }
}

impl WorldResidency {
    /// Packed building-badge icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_badge_glyphs(&self) -> Vec<u8> {
        self.draw_buffers.badge_glyph_buf.clone()
    }
}

impl WorldResidency {
    /// Tree lane off.
    #[must_use]
    pub fn tree_lane_off(&self) -> bool {
        !self.draw_buffers.tree_want || self.draw_buffers.heatmap_trees
    }
}

impl WorldResidency {
    /// Prop lane off.
    #[must_use]
    pub fn prop_lane_off(&self) -> bool {
        !self.draw_buffers.prop_want
    }
}

impl WorldResidency {
    /// Badge lane off.
    #[must_use]
    pub fn badge_lane_off(&self) -> bool {
        !self.draw_buffers.badge_want
    }
}

impl WorldResidency {
    /// Tree glyph count.
    #[must_use]
    pub fn tree_glyph_count(&self) -> u32 {
        (self.draw_buffers.tree_glyph_buf.len() / label_layout::glyph_math::ICON_INSTANCE_STRIDE)
            as u32
    }
}

impl WorldResidency {
    /// Prop glyph count.
    #[must_use]
    pub fn prop_glyph_count(&self) -> u32 {
        (self.draw_buffers.prop_glyph_buf.len() / label_layout::glyph_math::ICON_INSTANCE_STRIDE)
            as u32
    }
}

impl WorldResidency {
    /// Badge glyph count.
    #[must_use]
    pub fn badge_glyph_count(&self) -> u32 {
        (self.draw_buffers.badge_glyph_buf.len() / label_layout::glyph_math::ICON_INSTANCE_STRIDE)
            as u32
    }
}

impl WorldResidency {
    /// Test/diagnostic: glyph lookup entries for a compose group (0 tree, 1 prop, 2 building).
    #[cfg(test)]
    #[must_use]
    pub fn glyph_lookup_len_for_group(&self, group: u8) -> usize {
        self.draw_buffers
            .glyph_by_u16
            .values()
            .filter(|info| info.group == group)
            .count()
    }
}

impl WorldResidency {
    /// Test/diagnostic: atlas index for an icon key (when registered).
    #[cfg(test)]
    #[must_use]
    pub fn glyph_idx_for_key(&self, key: &str) -> Option<u16> {
        self.draw_buffers.icon_key_to_idx.get(key).copied()
    }
}

impl WorldResidency {
    /// Sorted draw-set chunk ids (strict visible ∩ pinned ∩ cells).
    #[must_use]
    pub fn draw_ids(&self) -> &[ChunkId] {
        &self.draw_buffers.draw_ids
    }
}

impl WorldResidency {
    /// Chunks draw.
    #[must_use]
    pub fn chunks_draw(&self) -> u32 {
        self.draw_buffers.draw_ids.len() as u32
    }
}

impl WorldResidency {
    /// Exact-count tree heatmap rung active.
    #[must_use]
    pub fn heatmap_trees_active(&self) -> bool {
        self.draw_buffers.heatmap_trees
    }
}

impl WorldResidency {
    /// Forest fill effective.
    #[must_use]
    pub fn forest_fill_effective(&self) -> bool {
        if class_visible("forestFill", self.chunk_residency.deck_zoom()) {
            return true;
        }
        self.draw_buffers.toggle_trees
            && (self.draw_buffers.heatmap_trees || self.draw_buffers.tree_glyph_buf.is_empty())
    }
}

impl WorldResidency {
    /// Exact tree count draw.
    #[must_use]
    pub fn exact_tree_count_draw(&self) -> u32 {
        self.draw_buffers.exact_tree_count
    }
}

impl WorldResidency {
    /// Buffers revision.
    #[must_use]
    pub fn buffers_revision(&self) -> u64 {
        self.draw_buffers.buffers_revision
    }
}
