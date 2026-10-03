//! The prefab occluder library: what the world line of sight knows of each catalogue prefab's
//! collision meshes, and the archive the browser boots it from.
//!
//! **Role:** declares the descriptor of one prefab ([`PrefabDescriptor`]), the library index
//! ([`BlasManifest`] with its rows and census), the 8-aligned building archive holder and its boot
//! split ([`BuildingArchiveBytes`], [`ArchiveBoot`]), and the projections between the JSON rows and
//! the archive rows.
//! **Position:** read by the world occluder ([`crate::residency`] expands descriptors), by the map
//! engine's occluder loader, which boots from the archive and fetches the manifest and descriptors,
//! and by the developer tools' blueprint tooling and library checks, which write and verify them.
//! **Signals & state:** none; plain data and pure conversions.
//! **Invariants:** `blocks` is true exactly when `localBounds` is present; an archive of another
//! schema version is refused, never read; a row whose BLAS list does not fully resolve is dropped
//! whole.

mod archive_projection;
mod blas_manifest;
mod building_archive;
mod prefab_descriptor;

pub use blas_manifest::{
    BlasEntry, BlasManifest, DESCRIPTOR_SCHEMA_VERSION, DescEntry, KindTotals,
    MANIFEST_SCHEMA_VERSION, Totals,
};
pub use building_archive::{ArchiveBoot, ArchiveProjectionError, BuildingArchiveBytes};
pub use prefab_descriptor::PrefabDescriptor;

#[cfg(test)]
#[path = "tests/occluder_library_tests.rs"]
mod tests;
