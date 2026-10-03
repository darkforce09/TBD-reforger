//! **Role:** the compute cull as the engine reports and draws it: the cull's switches and counters
//! for the bench hooks, and the indirect-draw list compacted across the fixed icon lane list.
//! **Position:** the map renderer, over the symbology layers' `IconCullGpu`; `encode.rs` collects
//! the indirect draws, the statistics read the per-lane counts, and the Mission Creator registers
//! the switches and counters on `window.__editorBench`.
//! **Signals & state:** none of its own; the cull's state is the `icon_cull` layer.
//! **Invariants:** this is GPU bookkeeping, not a spatial query: it walks a fixed nine-lane list and
//! hands the compacted buffers to the packet; a lane whose atlas is not uploaded is never drawn.

use crate::engine::RenderEngine;
use gpu_frame::frame::IndirectDraw;
use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use renderer_core::packet_bindings;
use symbology_layers_gpu::icon_uniforms::sprite_atlas_for;

impl RenderEngine {
    /// Whether the compute cull of the sprite lanes runs (WebGPU only).
    #[must_use]
    pub fn compute_cull_enabled(&self) -> bool {
        self.icon_cull.enabled()
    }

    /// Turn the compute cull's debug readout (the CPU oracle count of each cull) on or off.
    pub fn set_compute_cull_debug_hud(&mut self, on: bool) {
        self.icon_cull.set_debug_hud(on);
    }

    /// Class R CPU oracle count for the last encode_cull frustum (0 if the debug HUD flag is off).
    #[must_use]
    pub fn compute_cull_cpu_count(&self) -> u32 {
        self.icon_cull.last_cpu_count()
    }

    /// The sprite instances the last GPU cull readback kept, summed over the culled lanes.
    #[must_use]
    pub fn compute_cull_gpu_count(&self) -> u32 {
        self.icon_cull.gpu_count()
    }

    /// True once at least one real GPU counter readback has landed.
    #[must_use]
    pub fn compute_cull_gpu_sampled(&self) -> bool {
        self.icon_cull.gpu_sampled()
    }

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
        self.icon_cull.count_tree_icons_in_frustum(frustum)
    }
}

impl RenderEngine {
    /// Fill `out` with the indirect draws of the culled sprite lanes whose atlas is uploaded,
    /// sorted by lane.
    ///
    /// An out-param rather than a return, so this reads like its two siblings in `encode.rs`. Unlike them, `out` cannot be a `RenderEngine` field: `IndirectDraw<'a>`
    /// carries `&'a wgpu::Buffer` handles borrowed out of `self.icon_cull`, and a struct cannot
    /// hold a borrow of itself. What the shape does buy is the `reserve` below — one growth
    /// instead of up to three — and a call site where all three of the frame's tables are filled
    /// the same way.
    pub(crate) fn collect_indirect_icons<'a>(&'a self, out: &mut Vec<IndirectDraw<'a>>) {
        out.clear();
        if !self.icon_cull.enabled() {
            return;
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
        out.reserve(ROLES.len());
        for role in ROLES {
            let Some((dst, indirect)) = self.icon_cull.lane_draw(role as u32) else {
                continue;
            };

            // The atlas switch is the symbology layers' `sprite_atlas_for`, the table the lane
            // uploads read too. A lane whose atlas has not been uploaded is skipped here, so it
            // never reaches the packet at all.
            let atlas = sprite_atlas_for(role);
            let present = match atlas {
                packet_bindings::BIND_MOVABLE_SPRITE_ATLAS
                | packet_bindings::BIND_MOVABLE_SPRITE_ATLAS_DRAGGED => {
                    self.slot_symbology.has_atlas()
                }
                _ => self.glyph_atlas.is_uploaded(),
            };
            if !present {
                continue;
            }
            out.push(IndirectDraw {
                lane: lane_id(role),
                pipeline: packet_bindings::PIPE_ICON_STORAGE32,
                atlas,
                instances: dst,
                indirect,
            });
        }
        out.sort_by_key(|d| d.lane);
    }
}

impl RenderEngine {
    /// The instance count the statistics report for `role`'s lane: the compute cull's surviving
    /// count when the cull is in use, else `batch_sum`.
    pub(crate) fn cull_lane_stat(&self, role: LaneRole, batch_sum: u32) -> u32 {
        if self.icon_cull.enabled() {
            self.icon_cull.lane_gpu_count(role as u32)
        } else {
            batch_sum
        }
    }
}
