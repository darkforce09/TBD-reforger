//! T-090.12 — the world-scale occluder: every placed object on the terrain as a BLAS instance
//! under its chunk row's transform, culled by a per-chunk TLAS and walked chunk by chunk along a
//! segment. This module grows across the program:
//!
//! - `descriptor` (T-090.12.2): the per-prefab descriptor + library manifest wire types the
//!   offline emitter (`cargo xtask map bvh-batch --all-prefabs`), the verify lane, the CLI and
//!   the SPA all read through ONE definition.
//! - the occluder itself (T-090.12.3): TLAS, chunk DDA, descriptor expansion, proxy fallback.

pub mod descriptor;

pub use descriptor::{
    BlasEntry, BlasManifest, Bounds3, DESCRIPTOR_SCHEMA_VERSION, DescEntry, KindTotals,
    MANIFEST_SCHEMA_VERSION, PrefabDescriptor, Totals,
};
