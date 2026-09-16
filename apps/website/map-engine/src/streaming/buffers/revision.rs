//! Role: revision.
//! Position: `streaming/buffers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::culling::lod::class_visible;

use crate::streaming::scheduler::state::ResidencyEvent;
use crate::streaming::scheduler::state::WorldResidency;

/// Building-footprint LOD gate (`lodGates.ts` `BUILDING_FOOTPRINT_MIN_ZOOM`; manifest agrees).
pub const BUILDING_MIN_ZOOM: f64 = -2.5;

/// Norm.
pub(crate) fn norm(c: [u8; 4]) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        f32::from(c[3]) / 255.0,
    ]
}

impl WorldResidency {
    /// Building fill instances (WORLD coords): 10 f32 each `[x, y, hx, hy, cos, sin, r, g, b, a]`.
    #[must_use]
    pub fn world_building_fill(&self) -> Vec<f32> {
        self.fill_buf.clone()
    }
}

impl WorldResidency {
    /// Building outline vertices (WORLD coords): 6 f32 each `[x, y, r, g, b, a]`, `LineList`.
    #[must_use]
    pub fn world_building_outline(&self) -> Vec<f32> {
        self.outline_buf.clone()
    }
}

impl WorldResidency {
    /// World fence strips.
    #[must_use]
    pub fn world_fence_strips(&self) -> Vec<f32> {
        self.strip_buf.clone()
    }
}

impl WorldResidency {
    /// Fence strip segment count.
    #[must_use]
    pub fn fence_strip_segment_count(&self) -> u32 {
        self.fence_strip_count
    }
}

impl WorldResidency {
    /// Pier strip segment count.
    #[must_use]
    pub fn pier_strip_segment_count(&self) -> u32 {
        self.pier_strip_count
    }
}

impl WorldResidency {
    /// Bridge rail strip count.
    #[must_use]
    pub fn bridge_rail_strip_count(&self) -> u32 {
        self.bridge_rail_count
    }
}

impl WorldResidency {
    /// Packed tree+vegetation icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_tree_glyphs(&self) -> Vec<u8> {
        self.tree_glyph_buf.clone()
    }
}

impl WorldResidency {
    /// Packed prop+rockLarge icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_prop_glyphs(&self) -> Vec<u8> {
        self.prop_glyph_buf.clone()
    }
}

impl WorldResidency {
    /// Packed building-badge icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_badge_glyphs(&self) -> Vec<u8> {
        self.badge_glyph_buf.clone()
    }
}

impl WorldResidency {
    /// Tree lane off.
    #[must_use]
    pub fn tree_lane_off(&self) -> bool {
        !self.tree_want || self.heatmap_trees
    }
}

impl WorldResidency {
    /// Prop lane off.
    #[must_use]
    pub fn prop_lane_off(&self) -> bool {
        !self.prop_want
    }
}

impl WorldResidency {
    /// Badge lane off.
    #[must_use]
    pub fn badge_lane_off(&self) -> bool {
        !self.badge_want
    }
}

impl WorldResidency {
    /// Tree glyph count.
    #[must_use]
    pub fn tree_glyph_count(&self) -> u32 {
        (self.tree_glyph_buf.len()
            / crate::overlay::symbology::labels::glyph_math::ICON_INSTANCE_STRIDE) as u32
    }
}

impl WorldResidency {
    /// Prop glyph count.
    #[must_use]
    pub fn prop_glyph_count(&self) -> u32 {
        (self.prop_glyph_buf.len()
            / crate::overlay::symbology::labels::glyph_math::ICON_INSTANCE_STRIDE) as u32
    }
}

impl WorldResidency {
    /// Badge glyph count.
    #[must_use]
    pub fn badge_glyph_count(&self) -> u32 {
        (self.badge_glyph_buf.len()
            / crate::overlay::symbology::labels::glyph_math::ICON_INSTANCE_STRIDE) as u32
    }
}

impl WorldResidency {
    /// Test/diagnostic: glyph lookup entries for a compose group (0 tree, 1 prop, 2 building).
    #[cfg(test)]
    #[must_use]
    pub fn glyph_lookup_len_for_group(&self, group: u8) -> usize {
        self.glyph_by_u16
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
        self.icon_key_to_idx.get(key).copied()
    }
}

impl WorldResidency {
    /// Sorted draw-set chunk ids (strict visible ∩ pinned ∩ cells).
    #[must_use]
    pub fn draw_ids(&self) -> &[String] {
        &self.draw_ids
    }
}

impl WorldResidency {
    /// Chunks draw.
    #[must_use]
    pub fn chunks_draw(&self) -> u32 {
        self.draw_ids.len() as u32
    }
}

impl WorldResidency {
    /// Exact-count tree heatmap rung active.
    #[must_use]
    pub fn heatmap_trees_active(&self) -> bool {
        self.heatmap_trees
    }
}

impl WorldResidency {
    /// Forest fill effective.
    #[must_use]
    pub fn forest_fill_effective(&self) -> bool {
        if class_visible("forestFill", self.deck_zoom) {
            return true;
        }
        self.toggle_trees && (self.heatmap_trees || self.tree_glyph_buf.is_empty())
    }
}

impl WorldResidency {
    /// Exact tree count draw.
    #[must_use]
    pub fn exact_tree_count_draw(&self) -> u32 {
        self.exact_tree_count
    }
}

impl WorldResidency {
    /// Take residency events.
    pub fn take_residency_events(&mut self) -> Vec<ResidencyEvent> {
        std::mem::take(&mut self.residency_events)
    }
}

impl WorldResidency {
    /// Resident chunk ids (sorted) — parity/debug surface.
    #[must_use]
    pub fn resident_chunk_ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.chunks.keys().cloned().collect();
        v.sort();
        v
    }
}

impl WorldResidency {
    /// Total building instances across the pinned chunks (== JS `getWorldBuildings().length`).
    #[must_use]
    pub fn pinned_building_count(&self) -> u32 {
        self.pinned_ids
            .iter()
            .map(|id| self.building_counts.get(id).copied().unwrap_or(0))
            .sum()
    }
}

impl WorldResidency {
    /// Chunks resident.
    #[must_use]
    pub fn chunks_resident(&self) -> usize {
        self.chunks.len()
    }
}

impl WorldResidency {
    /// Instance count of a resident chunk (`None` if not resident).
    #[must_use]
    pub fn resident_instance_count(&self, id: &str) -> Option<u32> {
        self.chunks.get(id).map(|c| c.count)
    }
}

impl WorldResidency {
    /// Buffers revision.
    #[must_use]
    pub fn buffers_revision(&self) -> u64 {
        self.buffers_revision
    }
}
