//! Role: Module boundary for spatial/world_los/descriptor.
//! Position: `spatial/los/world/descriptor` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::archives::blueprints::ArchivedBlasEntry;
use crate::io::archives::blueprints::ArchivedBuildingBlueprintArchive;
use crate::io::archives::blueprints::ArchivedOccluderDescriptor;
use crate::io::archives::blueprints::BlasEntry as WireBlasEntry;
use crate::io::archives::blueprints::BuildingBlueprintArchive;
use crate::io::archives::blueprints::OccluderDescriptor as WireDescriptor;
use crate::io::archives::codec::BinaryError;
use crate::io::archives::codec::access_checked;
use crate::io::archives::version::ARCHIVE_SCHEMA_VERSION;
use crate::world::architecture::compound::instances::InstanceRecord;
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
