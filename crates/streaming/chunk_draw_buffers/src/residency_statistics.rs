//! The world residency's counters as one JSON object.
//!
//! **Role:** renders the residency's statistics (resident, pinned and applied chunks, apply
//! frames and times, building and index counts, in-flight and settle state, draw set, tree
//! counts, buffer revision and recompose counts, known-empty chunks) as an 18-key JSON object.
//! **Position:** `chunk_draw_buffers`; a method of
//! [`crate::world_residency::WorldResidency`]; read by the world loader's
//! statistics merge and the residency tests.
//! **Signals & state:** none; reads the chunk residency through its accessors and the draw
//! buffers' counters.
//! **Invariants:** the key set and order are fixed; every value is the counter it names.

use crate::world_residency::WorldResidency;

impl WorldResidency {
    /// Stats json.
    #[must_use]
    pub fn stats_json(&self) -> String {
        let residency = &self.chunk_residency;
        let buffers = &self.draw_buffers;
        format!(
            "{{\"chunks_resident\":{},\"chunks_pinned\":{},\"chunks_applied\":{},\"apply_frames\":{},\"apply_budget_ms_last\":{},\"max_apply_ms\":{},\"frames_over_budget\":{},\"building_instances\":{},\"index_size\":{},\"inflight_count\":{},\"pin_settled\":{},\"chunks_draw\":{},\"exact_tree_count\":{},\"heatmap_trees\":{},\"buffers_revision\":{},\"glyph_recomposes\":{},\"fill_recomposes\":{},\"known_empty_count\":{}}}",
            residency.chunks_resident(),
            residency.pinned_ids().len(),
            residency.chunks_applied(),
            residency.apply_frames(),
            residency.apply_budget_ms_last(),
            residency.max_apply_ms(),
            residency.frames_over_budget(),
            residency.pinned_building_count(),
            residency.object_index_size(),
            residency.inflight_count(),
            residency.pin_settled(),
            buffers.draw_ids.len(),
            buffers.exact_tree_count,
            buffers.heatmap_trees,
            buffers.buffers_revision,
            buffers.glyph_recomposes,
            buffers.fill_recomposes,
            residency.known_empty_count(),
        )
    }
}
