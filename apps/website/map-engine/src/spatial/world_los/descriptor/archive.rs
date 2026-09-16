//! Role: archive.
//! Position: `spatial/world_los/descriptor` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::ARCHIVE_SCHEMA_VERSION;
use super::ArchivedBuildingBlueprintArchive;
use super::BinaryError;
use super::BuildingBlueprintArchive;
use super::PrefabDescriptor;
use super::access_checked;

/// Archive projection error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchiveProjectionError {
    /// A BLAS path the descriptor names is absent from the library index the archive carries.
    UnknownBlas { prefab_id: u32, path: String },

    /// `blocks` and `localBounds` disagree. All 1623 committed descriptors satisfy `blocks == localBounds.is_some()`; the archive's non-optional `local_bounds` relies on it.
    BoundsDisagreeWithBlocks { prefab_id: u32, blocks: bool },
}

impl std::fmt::Display for ArchiveProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownBlas { prefab_id, path } => write!(
                f,
                "descriptor {prefab_id}: BLAS {path} is not in the archive's library index"
            ),
            Self::BoundsDisagreeWithBlocks { prefab_id, blocks } => write!(
                f,
                "descriptor {prefab_id}: blocks = {blocks} but localBounds.is_some() = {}; the \
                 archive's local_bounds is not optional and would be written as a zero box",
                !blocks
            ),
        }
    }
}

impl core::error::Error for ArchiveProjectionError {}

/// Building archive bytes.
pub struct BuildingArchiveBytes {
    /// Words.
    pub(super) words: Vec<u64>,

    /// Len.
    pub(super) len: usize,
}

impl BuildingArchiveBytes {
    /// Copy `src` into an 8-aligned buffer.
    #[must_use]
    pub fn new(src: &[u8]) -> Self {
        let mut words = vec![0u64; src.len().div_ceil(8)];
        for (w, c) in words.iter_mut().zip(src.chunks(8)) {
            let mut b = [0u8; 8];
            b[..c.len()].copy_from_slice(c);
            *w = u64::from_le_bytes(b);
        }
        Self {
            words,
            len: src.len(),
        }
    }
}

impl BuildingArchiveBytes {
    /// The archive bytes, 8-aligned.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &bytemuck::cast_slice(&self.words)[..self.len]
    }
}

impl BuildingArchiveBytes {
    /// The validated archive, borrowed in place — no deserialise, no allocation.
    pub fn archive(&self) -> Result<&ArchivedBuildingBlueprintArchive, BinaryError> {
        let archive = access_checked::<BuildingBlueprintArchive>(self.as_slice())?;
        let version = archive.schema_version.to_native();
        if version != ARCHIVE_SCHEMA_VERSION {
            return Err(BinaryError::UnsupportedVersion {
                what: "BuildingBlueprintArchive",
                expected: ARCHIVE_SCHEMA_VERSION,
                actual: version,
            });
        }
        Ok(archive)
    }
}

/// [`blas_by_pid`](Self::blas_by_pid) is the half that *is* complete for every prefab: the `.bvh` fetch list of every row, resolved through the archive's shared library. It is what lets a loader queue a prefab's sidecars in the same round as its descriptor instead of the round after.
pub struct ArchiveBoot {
    /// Rebuilt `blocks: false` descriptors — safe to insert, and the reason those pids are never fetched again (`insert_descriptor` routes them to `no_block`).
    pub census: Vec<PrefabDescriptor>,

    /// `(pid, .bvh paths)` for **every** archived row, pid-ascending, paths in first-use order.
    pub blas_by_pid: Vec<(u16, Vec<String>)>,

    /// Archived rows with `blocks: true` — the descriptors a loader must still read as JSON.
    pub blocking: usize,

    /// Rows whose `prefab_id` does not fit `u16`, or whose BLAS list does not resolve against `blas_index`. Both are dropped rather than guessed, and counted so a loader can say so.
    pub unusable: usize,
}

impl ArchiveBoot {
    /// Split a validated archive. Cheap enough to run once at boot: one pass over the rows.
    #[must_use]
    pub fn from_archive(a: &ArchivedBuildingBlueprintArchive) -> Self {
        let mut boot = Self {
            census: Vec::new(),
            blas_by_pid: Vec::with_capacity(a.descriptors.len()),
            blocking: 0,
            unusable: 0,
        };
        for row in a.descriptors.iter() {
            let (Ok(pid), Some(paths)) = (
                u16::try_from(row.prefab_id.to_native()),
                PrefabDescriptor::archived_blas_paths(row, &a.blas_index),
            ) else {
                boot.unusable += 1;
                continue;
            };
            boot.blas_by_pid.push((pid, paths));
            if row.blocks {
                boot.blocking += 1;
            } else {
                boot.census.push(PrefabDescriptor::from_archived(row));
            }
        }
        boot.blas_by_pid.sort_by_key(|(pid, _)| *pid);
        boot
    }
}
