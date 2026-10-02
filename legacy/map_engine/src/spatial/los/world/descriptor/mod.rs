//! Role: Module boundary for spatial/world_los/descriptor.
//! Position: `spatial/los/world/descriptor` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::architecture::compound::instances::InstanceRecord;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use world_file_formats::archives::blueprints::ArchivedBlasEntry;
use world_file_formats::archives::blueprints::ArchivedBuildingBlueprintArchive;
use world_file_formats::archives::blueprints::ArchivedOccluderDescriptor;
use world_file_formats::archives::blueprints::BlasEntry as WireBlasEntry;
use world_file_formats::archives::blueprints::BuildingBlueprintArchive;
use world_file_formats::archives::blueprints::OccluderDescriptor as WireDescriptor;
use world_file_formats::archives::codec::BinaryError;
use world_file_formats::archives::codec::access_checked;
use world_file_formats::archives::version::ARCHIVE_SCHEMA_VERSION;

mod manifest;
#[cfg(test)]
#[path = "../tests/descriptor_tests.rs"]
mod tests;

/// Re-export `manifest::{BlasEntry,BlasManifest,DESCRIPTOR_SCHEMA_VERSION,DescEntry,KindTotals,MANIFEST_SCHEMA_VERSION,Totals,}`.
pub use manifest::{
    BlasEntry, BlasManifest, DESCRIPTOR_SCHEMA_VERSION, DescEntry, KindTotals,
    MANIFEST_SCHEMA_VERSION, Totals,
};

mod model;

/// Re-export `model::PrefabDescriptor`.
pub use model::PrefabDescriptor;
mod archive;
mod projection;

/// Re-export `archive::{ArchiveBoot,ArchiveProjectionError,BuildingArchiveBytes}`.
pub use archive::{ArchiveBoot, ArchiveProjectionError, BuildingArchiveBytes};
