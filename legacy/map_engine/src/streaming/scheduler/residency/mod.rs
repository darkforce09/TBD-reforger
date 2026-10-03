//! Role: Module boundary for streaming/scheduler/residency.
//! Position: `streaming/scheduler/residency` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

#[cfg(test)]
use prefab_catalog::footprint_lookups::building_prefab_lookup;
#[cfg(test)]
use prefab_catalog::footprint_lookups::fence_prefab_lookup;

#[cfg(test)]
use prefab_catalog::prefab_rows::narrow_prefab_rows;

/// Re-export `crate::streaming::scheduler::viewport::DRAW_CULL_MARGIN_M`.
pub use crate::streaming::scheduler::viewport::DRAW_CULL_MARGIN_M;

/// Re-export `crate::streaming::scheduler::viewport::FETCH_FAILURE_CAP`.
pub use crate::streaming::scheduler::viewport::FETCH_FAILURE_CAP;

/// Re-export `crate::streaming::scheduler::viewport::LRU_MIN_CHUNKS`.
pub use crate::streaming::scheduler::viewport::LRU_MIN_CHUNKS;

/// Re-export `crate::streaming::scheduler::state::IngestOutcome`.
pub use crate::streaming::scheduler::state::IngestOutcome;

/// Re-export `crate::streaming::scheduler::state::ResidencyEvent`.
pub use crate::streaming::scheduler::state::ResidencyEvent;

/// Re-export `crate::streaming::scheduler::state::WorldResidency`.
pub use crate::streaming::scheduler::state::WorldResidency;

/// Re-export `crate::streaming::scheduler::budget::APPLY_BUDGET_MS`.
pub use crate::streaming::scheduler::budget::APPLY_BUDGET_MS;

#[cfg(test)]
mod t151_11_3_tests;
#[cfg(test)]
mod t152_3_tests;
#[cfg(test)]
mod tests;
