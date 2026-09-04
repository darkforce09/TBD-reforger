//! T-090.12 — the world-scale occluder: every placed object on the terrain as a BLAS instance
//! under its chunk row's transform, culled by a per-chunk TLAS and walked chunk by chunk along a
//! segment.
//!
//! - `descriptor` (T-090.12.2): the per-prefab descriptor + library manifest wire types the
//!   offline emitter (`cargo xtask map bvh-batch --all-prefabs`), the verify lane, the CLI and
//!   the SPA all read through ONE definition.
//! - `tlas` / `dda` / `placed` / `trace` (T-090.12.3): the AABB tree over a chunk's rows, the
//!   chunk walk, the rows themselves, and [`WorldOccluder`] — trace / blocked / evaluate_los with
//!   the compound's material semantics and honest coverage.

pub mod dda;
pub mod descriptor;
mod los;
pub mod placed;
pub mod tlas;
pub mod trace;

pub use dda::cells_on_segment;
pub use descriptor::{
    BlasEntry, BlasManifest, Bounds3, DESCRIPTOR_SCHEMA_VERSION, DescEntry, KindTotals,
    MANIFEST_SCHEMA_VERSION, PrefabDescriptor, Totals,
};
pub use placed::{ChunkOccluder, WorldInstance, rows_of_chunk};
pub use tlas::{AabbTlas, Candidate};
pub use trace::{
    BlockPolicy, Coverage, DEFAULT_BLAS_CAP_BYTES, Fidelity, PrefabOccluder, Wanted, WorldEvent,
    WorldLos, WorldOccluder, WorldVerdict, map_to_engine,
};

#[cfg(test)]
#[path = "occluder_tests.rs"]
mod tests;
