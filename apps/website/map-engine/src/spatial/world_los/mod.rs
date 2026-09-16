//! Role: Module boundary for spatial/world_los.
//! Position: `spatial/world_los` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Dda.
pub mod dda;

/// Descriptor.
pub mod descriptor;
mod los;

/// Placed.
pub mod placed;

/// Tlas.
pub mod tlas;

/// Trace.
pub mod trace;

/// Re-export `crate::spatial::world_los::coverage_1::BlockPolicy`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  block policy at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::BlockPolicy;

/// Re-export `crate::spatial::world_los::coverage_1::Coverage`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  coverage at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::Coverage;

/// Re-export `crate::spatial::world_los::coverage_1::DEFAULT_BLAS_CAP_BYTES`.
#[doc = " Expose crate :: spatial :: world los :: trace :: default blas cap bytes at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::DEFAULT_BLAS_CAP_BYTES;

/// Re-export `crate::spatial::world_los::coverage_1::Fidelity`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  fidelity at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::Fidelity;

/// Re-export `crate::spatial::world_los::coverage_1::PrefabOccluder`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  prefab occluder at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::PrefabOccluder;

/// Re-export `crate::spatial::world_los::coverage_1::Wanted`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  wanted at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::Wanted;

/// Re-export `crate::spatial::world_los::coverage_1::WorldEvent`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  world event at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::WorldEvent;

/// Re-export `crate::spatial::world_los::coverage_1::WorldLos`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  world los at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::WorldLos;

/// Re-export `crate::spatial::world_los::coverage_1::WorldVerdict`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  world verdict at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::WorldVerdict;

/// Re-export `crate::spatial::world_los::coverage_1::map_to_engine`.
#[doc = " Expose crate :: spatial :: world los :: trace :: map to engine at this domain boundary."]
pub use crate::spatial::world_los::coverage_1::map_to_engine;

/// Re-export `crate::spatial::world_los::dda::cells_on_segment`.
pub use crate::spatial::world_los::dda::cells_on_segment;

/// Re-export `crate::spatial::world_los::descriptor::BlasEntry`.
pub use crate::spatial::world_los::descriptor::BlasEntry;

/// Re-export `crate::spatial::world_los::descriptor::BlasManifest`.
pub use crate::spatial::world_los::descriptor::BlasManifest;

/// Re-export `crate::spatial::world_los::descriptor::Bounds3`.
pub use crate::spatial::world_los::descriptor::Bounds3;

/// Re-export `crate::spatial::world_los::descriptor::DESCRIPTOR_SCHEMA_VERSION`.
pub use crate::spatial::world_los::descriptor::DESCRIPTOR_SCHEMA_VERSION;

/// Re-export `crate::spatial::world_los::descriptor::DescEntry`.
pub use crate::spatial::world_los::descriptor::DescEntry;

/// Re-export `crate::spatial::world_los::descriptor::KindTotals`.
pub use crate::spatial::world_los::descriptor::KindTotals;

/// Re-export `crate::spatial::world_los::descriptor::MANIFEST_SCHEMA_VERSION`.
pub use crate::spatial::world_los::descriptor::MANIFEST_SCHEMA_VERSION;

/// Re-export `crate::spatial::world_los::descriptor::PrefabDescriptor`.
pub use crate::spatial::world_los::descriptor::PrefabDescriptor;

/// Re-export `crate::spatial::world_los::descriptor::Totals`.
pub use crate::spatial::world_los::descriptor::Totals;

/// Re-export `crate::spatial::world_los::placed::ChunkOccluder`.
pub use crate::spatial::world_los::placed::ChunkOccluder;

/// Re-export `crate::spatial::world_los::placed::WorldInstance`.
pub use crate::spatial::world_los::placed::WorldInstance;

/// Re-export `crate::spatial::world_los::placed::rows_of_chunk`.
pub use crate::spatial::world_los::placed::rows_of_chunk;

/// Re-export `crate::spatial::world_los::state::WorldOccluder`.
#[doc = " Expose crate :: spatial :: world los :: trace ::  world occluder at this domain boundary."]
pub use crate::spatial::world_los::state::WorldOccluder;

/// Re-export `crate::spatial::world_los::tlas::AabbTlas`.
pub use crate::spatial::world_los::tlas::AabbTlas;

/// Re-export `crate::spatial::world_los::tlas::Candidate`.
pub use crate::spatial::world_los::tlas::Candidate;

#[cfg(test)]
#[path = "tests/occluder.rs"]
mod tests;

/// Coverage 1.
#[cfg(feature = "streaming")]
pub mod coverage_1;

/// Coverage 2.
#[cfg(feature = "streaming")]
pub mod coverage_2;

/// State.
#[cfg(feature = "streaming")]
pub mod state;

/// Residency.
#[cfg(feature = "streaming")]
pub mod residency;

/// Raycast.
#[cfg(feature = "streaming")]
pub mod raycast;
