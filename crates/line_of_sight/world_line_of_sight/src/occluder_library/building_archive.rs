//! The building archive `prefabs/building_blueprints.rkyv` as the occluder boots from it.
//!
//! **Role:** holds the archive bytes 8-aligned ([`BuildingArchiveBytes`]), reads them as a
//! validated archive of the current schema in place, and splits its rows at boot
//! ([`ArchiveBoot`]): the never-blocking descriptors rebuilt, the count still read as JSON, and
//! every usable row's BLAS paths; declares why a descriptor cannot be projected into it
//! ([`ArchiveProjectionError`]).
//! **Position:** the map engine's occluder loader boots from it; the developer tools' archive
//! writer projects descriptors into it.
//! **Signals & state:** the holder owns its aligned copy of the bytes.
//! **Invariants:** an archive of another schema version is refused; a row whose prefab id does not
//! fit 16 bits or whose BLAS list does not resolve is dropped and counted, never guessed.

use world_file_formats::archives::blueprints::{
    ArchivedBuildingBlueprintArchive, BuildingBlueprintArchive,
};
use world_file_formats::archives::codec::{BinaryError, access_checked};
use world_file_formats::archives::version::ARCHIVE_SCHEMA_VERSION;
use world_file_formats::ids::PrefabId;

use crate::occluder_library::PrefabDescriptor;

/// Why a descriptor cannot be projected into the building archive.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ArchiveProjectionError {
    /// A BLAS path the descriptor names is absent from the library index the archive carries.
    #[error("descriptor {prefab_id}: BLAS {path} is not in the archive's library index")]
    UnknownBlas {
        /// The descriptor's catalogue id.
        prefab_id: PrefabId,

        /// The BLAS path the library index lacks.
        path: String,
    },

    /// `blocks` and `localBounds` disagree: the archive's `local_bounds` is not optional, so a
    /// blocking descriptor needs bounds and a non-blocking one must have none.
    #[error(
        "descriptor {prefab_id}: blocks = {blocks} but localBounds.is_some() = {}; the \
         archive's local_bounds is not optional and would be written as a zero box",
        !blocks
    )]
    BoundsDisagreeWithBlocks {
        /// The descriptor's catalogue id.
        prefab_id: PrefabId,

        /// The descriptor's `blocks` flag.
        blocks: bool,
    },
}

/// The archive bytes copied into an 8-aligned buffer, as rkyv needs to read them in place.
pub struct BuildingArchiveBytes {
    /// The bytes, as little-endian words.
    pub(super) words: Vec<u64>,

    /// The byte length.
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
    ///
    /// # Errors
    /// [`BinaryError`] when the bytes do not validate or the schema version is not
    /// [`ARCHIVE_SCHEMA_VERSION`].
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

/// The boot split of a validated archive: the census rows rebuilt, the count still read as JSON,
/// and [`blas_by_pid`](Self::blas_by_pid), the `.bvh` fetch list of every usable row resolved
/// through the archive's shared library.
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
                u16::try_from(row.prefab_id.get()),
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
