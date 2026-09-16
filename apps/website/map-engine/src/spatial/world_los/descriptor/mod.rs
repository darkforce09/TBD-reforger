//! Role: Module boundary for spatial/world_los/descriptor.
//! Position: `spatial/world_los/descriptor` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::compound::instances::InstanceRecord;
use crate::formats::archives::blueprints::ArchivedBlasEntry;
use crate::formats::archives::blueprints::ArchivedBuildingBlueprintArchive;
use crate::formats::archives::blueprints::ArchivedOccluderDescriptor;
use crate::formats::archives::blueprints::BlasEntry as WireBlasEntry;
use crate::formats::archives::blueprints::BuildingBlueprintArchive;
use crate::formats::archives::blueprints::OccluderDescriptor as WireDescriptor;
use crate::formats::archives::codec::BinaryError;
use crate::formats::archives::codec::access_checked;
use crate::formats::archives::version::ARCHIVE_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod manifest;
#[cfg(test)]
#[path = "../tests/descriptor_tests.rs"]
mod tests;

/// Re-export `manifest::{BlasEntry,BlasManifest,DESCRIPTOR_SCHEMA_VERSION,DescEntry,KindTotals,MANIFEST_SCHEMA_VERSION,Totals,}`.
pub use manifest::{
    BlasEntry, BlasManifest, DESCRIPTOR_SCHEMA_VERSION, DescEntry, KindTotals,
    MANIFEST_SCHEMA_VERSION, Totals,
};
mod bounds;

/// Re-export `bounds::Bounds3`.
pub use bounds::Bounds3;

mod model;

/// Re-export `model::PrefabDescriptor`.
pub use model::PrefabDescriptor;
mod archive;
mod projection;

/// Re-export `archive::{ArchiveBoot,ArchiveProjectionError,BuildingArchiveBytes}`.
pub use archive::{ArchiveBoot, ArchiveProjectionError, BuildingArchiveBytes};
