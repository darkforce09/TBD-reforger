//! Role: residency.
//! Position: `streaming/memory` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::scheduler::state::WorldResidency;

impl WorldResidency {
    /// Stats json.
    #[must_use]
    pub fn stats_json(&self) -> String {
        format!(
            "{{\"chunks_resident\":{},\"chunks_pinned\":{},\"chunks_applied\":{},\"apply_frames\":{},\"apply_budget_ms_last\":{},\"max_apply_ms\":{},\"frames_over_budget\":{},\"building_instances\":{},\"index_size\":{},\"inflight_count\":{},\"pin_settled\":{},\"chunks_draw\":{},\"exact_tree_count\":{},\"heatmap_trees\":{},\"buffers_revision\":{},\"glyph_recomposes\":{},\"fill_recomposes\":{},\"known_empty_count\":{}}}",
            self.chunks.len(),
            self.pinned_ids.len(),
            self.chunks_applied,
            self.apply_frames,
            self.apply_budget_ms_last,
            self.max_apply_ms,
            self.frames_over_budget,
            self.pinned_building_count(),
            self.index.size(),
            self.inflight.len(),
            self.pin_settled(),
            self.draw_ids.len(),
            self.exact_tree_count,
            self.heatmap_trees,
            self.buffers_revision,
            self.glyph_recomposes,
            self.fill_recomposes,
            self.known_empty.len(),
        )
    }
}
