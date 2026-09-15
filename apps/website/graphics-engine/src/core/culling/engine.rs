//! Role: engine.
//! Position: `core/culling` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::core::pipeline::draw_order::lane_order;
use crate::renderers::batching::batch::IndirectIcon;
use crate::renderers::batching::scene::ANCHOR;
use wasm_bindgen::prelude::*;

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
        crate::core::culling::oracle::count_icons_in_frustum(&self.tree_icons_20, frustum)
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
    pub(crate) fn collect_indirect_icons(&self) -> Vec<IndirectIcon<'_>> {
        let Some(cull) = self.icon_cull.as_ref() else {
            return Vec::new();
        };
        let Some(pipe32) = self.icon_pipeline_storage32.as_ref() else {
            return Vec::new();
        };
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
            let atlas = match role {
                LaneRole::SlotDrag => self.slot_atlas.as_ref().map(|a| &a.drag_bind_group),
                LaneRole::Slots
                | LaneRole::Clusters
                | LaneRole::SlotPlacePreview
                | LaneRole::MissionVehicles
                | LaneRole::MissionComments => self.slot_atlas.as_ref().map(|a| &a.base_bind_group),
                _ => self.glyph_atlas.as_ref().map(|a| &a.bind_group),
            };
            let Some(atlas_bind) = atlas else {
                continue;
            };
            out.push(IndirectIcon {
                role,
                pipeline: pipe32,
                atlas_bind,
                instances: dst,
                indirect,
            });
        }
        out.sort_by_key(|d| lane_order(d.role));
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
