//! **Role:** the lane-aware half of the frame packet's binding policy: the size of the bind-group
//! table.
//! **Position:** the map renderer, over `renderer_core::packet_bindings`, which numbers the
//! packet's pipeline and bind-group slots without knowing a lane; `encode.rs` sizes the table.
//! **Signals & state:** none; constants.
//! **Invariants:** every decision that depends on what a lane means (density versus plain textured
//! in the world layers' `world_layers_gpu::textured_quad`; which atlas a sprite lane samples in the
//! symbology layers' `sprite_atlas_for`) travels to the renderer as a `PipelineId` or
//! `BindGroupId` on the batch, so the encoder binds without asking what a lane is. The bind table
//! is sized from the widest lane id the lane list can produce.
//!
//! The world label lanes need no entry here: they build a `DrawPayload::Text` at upload time, so
//! the routing to the text pipeline is the payload itself.

use renderer_core::packet_bindings::BIND_TEX_BASE;

/// One bind-group slot per possible lane id, above the fixed atlas slots.
///
/// `map_draw_lanes::lane_roles::lane_id` is `2·lane_order` (+1 for the one tie), so the widest
/// id the 48 lanes can produce is `2·47`. Sizing the table from that keeps a textured lane's slot
/// (`renderer_core::packet_bindings::tex_bind_id`) a pure function of its lane: stable for the
/// life of the lane, with no allocator and no free list.
pub(crate) const BIND_SLOTS: usize = BIND_TEX_BASE as usize + 95;

// The `95` above is `2·(ALL_LANES.len() - 1) + 1`: the widest `lane_id` the 48 lanes can produce,
// plus one because the table is indexed by it. `ALL_LANES` lives in another crate,
// `map_draw_lanes::lane_roles`, and nothing but this arithmetic ties the two together: a 49th
// lane would produce `lane_id == 96`, index slot `5 + 96`, and run off the end of the table. The
// assert makes that a compile error at zero runtime cost. If it fires, widen `BIND_SLOTS`; do not
// delete the assert.
const _: () = assert!(
    BIND_SLOTS
        == BIND_TEX_BASE as usize + 2 * (map_draw_lanes::lane_roles::ALL_LANES.len() - 1) + 1,
    "BIND_SLOTS must index the widest lane_id: base + 2·(ALL_LANES.len() - 1), inclusive"
);
