//! Role: indirect-draw compaction across the engine's fixed lane list.
//! Position: `frame` in the map engine.
//! Signals & state: per-lane `IndirectDraw` counts for the compute-cull path.
//! Invariants: this is GPU bookkeeping, not a spatial query. It walks a hardcoded nine-lane
//! list and writes indirect-draw arguments; the frustum arithmetic it calls is
//! `website_graphics_engine::draw::cull`, which crossed in Phase 1.
//!
//! T-0xx Phase 2B.1: from `core/culling/engine.rs`. It is filed under `frame/` and **not**
//! `spatial/` for exactly that reason — `spatial/` answers questions about the world, and
//! nothing here does.

use crate::frame::bindings;
use crate::frame::engine::RenderEngine;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;
use crate::world::scene::ANCHOR;
use wasm_bindgen::prelude::*;
use website_graphics_engine::frame::IndirectDraw;

#[wasm_bindgen]
impl RenderEngine {
    /// Compute cull enabled.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn compute_cull_enabled(&self) -> bool {
        self.gpu_cull_enabled()
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set compute cull debug hud.
    pub fn set_compute_cull_debug_hud(&mut self, on: bool) {
        if let Some(cull) = &mut self.icon_cull {
            cull.set_debug_hud(on);
        }
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Class R CPU oracle count for the last encode_cull frustum (0 if the debug HUD flag is off).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn compute_cull_cpu_count(&self) -> u32 {
        self.icon_cull
            .as_ref()
            .map(|c| c.last_cpu_count())
            .unwrap_or(0)
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Compute cull gpu count.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn compute_cull_gpu_count(&self) -> u32 {
        self.icon_cull
            .as_ref()
            .map(|c| c.gpu_count_for_stats())
            .unwrap_or(0)
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// True once at least one real GPU counter readback has landed.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn compute_cull_gpu_sampled(&self) -> bool {
        self.icon_cull.as_ref().is_some_and(|c| c.gpu_sampled())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Pure CPU compact of current tree icons against a world-meter frustum (Class R harness). Returns surviving instance count. Frustum is WORLD meters (converted to anchor-relative).
    #[must_use]
    pub fn compute_cull_cpu_count_for_frustum(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> u32 {
        let frustum = [
            min_x - ANCHOR[0],
            min_y - ANCHOR[1],
            max_x - ANCHOR[0],
            max_y - ANCHOR[1],
        ];
        crate::frame::oracle::count_icons_in_frustum(&self.tree_icons_20, frustum)
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Gpu cull enabled.
    pub(crate) fn gpu_cull_enabled(&self) -> bool {
        self.compute_cull_trees
            && self.icon_cull.is_some()
            && self.icon_pipeline_storage32.is_some()
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Clear cull lane.
    pub(crate) fn clear_cull_lane(&mut self, role: LaneRole) {
        if let Some(cull) = &mut self.icon_cull {
            cull.upload_lane(&self.device, &self.queue, role as u32, &[]);
        }
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Collect indirect icons.
    pub(crate) fn collect_indirect_icons(&self) -> Vec<IndirectDraw<'_>> {
        let Some(cull) = self.icon_cull.as_ref() else {
            return Vec::new();
        };
        if self.icon_pipeline_storage32.is_none() {
            return Vec::new();
        }
        if !self.compute_cull_trees {
            return Vec::new();
        }
        const ROLES: [LaneRole; 9] = [
            LaneRole::WorldTrees,
            LaneRole::WorldProps,
            LaneRole::WorldBadges,
            LaneRole::MissionComments,
            LaneRole::MissionVehicles,
            LaneRole::Slots,
            LaneRole::SlotPlacePreview,
            LaneRole::SlotDrag,
            LaneRole::Clusters,
        ];
        let mut out = Vec::new();
        for role in ROLES {
            let Some((dst, indirect)) = cull.lane_draw(role as u32) else {
                continue;
            };

            // T-0xx Phase 1D: the atlas switch is `bindings::sprite_atlas_for` — the same
            // table `encoder.rs` used to hold a second copy of. A lane whose atlas has not
            // been uploaded is still skipped here, so it never reaches the packet at all.
            let atlas = bindings::sprite_atlas_for(role);
            let present = match atlas {
                bindings::BIND_SLOT_BASE | bindings::BIND_SLOT_DRAG => self.slot_atlas.is_some(),
                _ => self.glyph_atlas.is_some(),
            };
            if !present {
                continue;
            }
            out.push(IndirectDraw {
                lane: lane_id(role),
                pipeline: bindings::PIPE_ICON_STORAGE32,
                atlas,
                instances: dst,
                indirect,
            });
        }
        out.sort_by_key(|d| d.lane);
        out
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Cull lane stat.
    pub(crate) fn cull_lane_stat(&self, role: LaneRole, batch_sum: u32) -> u32 {
        if self.gpu_cull_enabled() {
            self.icon_cull
                .as_ref()
                .map(|c| c.lane_gpu_count_for_stats(role as u32))
                .unwrap_or(0)
        } else {
            batch_sum
        }
    }
}
