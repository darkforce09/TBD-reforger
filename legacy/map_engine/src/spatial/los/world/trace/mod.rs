//! Role: Module boundary for spatial/world_los/trace.
//! Position: `spatial/los/world/trace` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::spatial::los::world::coverage_1::BlockPolicy`.
pub use crate::spatial::los::world::coverage_1::BlockPolicy;

/// Re-export `crate::spatial::los::world::coverage_1::Coverage`.
pub use crate::spatial::los::world::coverage_1::Coverage;

/// Re-export `crate::spatial::los::world::coverage_1::DEFAULT_BLAS_CAP_BYTES`.
pub use crate::spatial::los::world::coverage_1::DEFAULT_BLAS_CAP_BYTES;

/// Re-export `crate::spatial::los::world::coverage_1::Fidelity`.
pub use crate::spatial::los::world::coverage_1::Fidelity;

/// Re-export `crate::spatial::los::world::coverage_1::PrefabOccluder`.
pub use crate::spatial::los::world::coverage_1::PrefabOccluder;

/// Re-export `crate::spatial::los::world::coverage_1::Wanted`.
pub use crate::spatial::los::world::coverage_1::Wanted;

/// Re-export `crate::spatial::los::world::coverage_1::WorldEvent`.
pub use crate::spatial::los::world::coverage_1::WorldEvent;

/// Re-export `crate::spatial::los::world::coverage_1::WorldLos`.
pub use crate::spatial::los::world::coverage_1::WorldLos;

/// Re-export `crate::spatial::los::world::coverage_1::WorldVerdict`.
pub use crate::spatial::los::world::coverage_1::WorldVerdict;

/// Re-export `crate::spatial::los::world::coverage_1::map_to_engine`.
pub use crate::spatial::los::world::coverage_1::map_to_engine;

/// Re-export `crate::spatial::los::world::state::WorldOccluder`.
pub use crate::spatial::los::world::state::WorldOccluder;
