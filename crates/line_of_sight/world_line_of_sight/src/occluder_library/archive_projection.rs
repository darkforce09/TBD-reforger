//! The projections between the library's JSON rows and the building archive's rows.
//!
//! **Role:** projects a [`crate::occluder_library::PrefabDescriptor`] into an archive row (refusing
//! rather than approximating), reduces it to what the archive carries, rebuilds one from a row,
//! resolves a row's BLAS paths through the library index, and converts BLAS rows both ways.
//! **Position:** the developer tools' archive writer projects into the archive; the archive's boot
//! split ([`crate::occluder_library::ArchiveBoot`]) and the library checks read back from it.
//! **Signals & state:** none; pure conversions.
//! **Invariants:** a descriptor read back from a row equals the original's `archive_census`; a BLAS
//! row round-trips losslessly; a row's BLAS list resolves whole or not at all.

use world_file_formats::archives::blueprints::{
    ArchivedBlasEntry, ArchivedOccluderDescriptor, BlasEntry as WireBlasEntry,
    OccluderDescriptor as WireDescriptor,
};

use crate::occluder_library::{
    ArchiveProjectionError, BlasEntry, DESCRIPTOR_SCHEMA_VERSION, PrefabDescriptor,
};
use geometry_primitives::axis_aligned_box::Bounds3;

impl PrefabDescriptor {
    /// This descriptor as an archive row, its BLAS paths as indexes `index_of` resolves into the
    /// library index.
    ///
    /// # Errors
    /// [`ArchiveProjectionError`] when `blocks` and `local_bounds` disagree or a BLAS path is not in
    /// the library index.
    pub fn to_archived(
        &self,
        index_of: &impl Fn(&str) -> Option<u32>,
    ) -> Result<WireDescriptor, ArchiveProjectionError> {
        if self.blocks != self.local_bounds.is_some() {
            return Err(ArchiveProjectionError::BoundsDisagreeWithBlocks {
                prefab_id: self.prefab_id,
                blocks: self.blocks,
            });
        }
        let mut blas = Vec::with_capacity(self.instances.len());
        for p in self.blas_paths() {
            let i = index_of(p).ok_or_else(|| ArchiveProjectionError::UnknownBlas {
                prefab_id: self.prefab_id,
                path: p.to_string(),
            })?;
            blas.push(i);
        }
        let b = self.local_bounds.unwrap_or(Bounds3 {
            min: [0.0; 3],
            max: [0.0; 3],
        });
        Ok(WireDescriptor {
            prefab_id: self.prefab_id,
            slug: self.slug.clone(),
            kind: self.kind.clone(),
            blocks: self.blocks,
            canopy: self.canopy,
            local_bounds: [b.min.map(|v| v as f32), b.max.map(|v| v as f32)],
            blas,
        })
    }
}

impl PrefabDescriptor {
    /// The descriptor reduced to exactly what the archive can carry — and the cleared fields ARE the documented loss, in code rather than in prose.
    #[must_use]
    pub fn archive_census(&self) -> Self {
        Self {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.to_string(),
            prefab_id: self.prefab_id,
            slug: self.slug.clone(),
            kind: self.kind.clone(),
            blocks: self.blocks,
            canopy: self.canopy,
            local_bounds: self.local_bounds.map(Bounds3::to_f32_lossy),
            resource_name: String::new(),
            reason: None,
            shell_bvh: String::new(),
            instances: Vec::new(),
            notes: Vec::new(),
        }
    }
}

impl PrefabDescriptor {
    /// The descriptor an archive row carries: id, slug, kind, flags and f32 bounds, every other
    /// field cleared.
    #[must_use]
    pub fn from_archived(a: &ArchivedOccluderDescriptor) -> Self {
        let blocks = a.blocks;
        let corner = |c: &[rkyv::rend::f32_le; 3]| {
            [
                f64::from(c[0].to_native()),
                f64::from(c[1].to_native()),
                f64::from(c[2].to_native()),
            ]
        };
        Self {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.to_string(),
            prefab_id: a.prefab_id.to_native(),
            slug: a.slug.to_string(),
            kind: a.kind.to_string(),
            blocks,
            canopy: a.canopy,
            local_bounds: blocks.then(|| Bounds3 {
                min: corner(&a.local_bounds[0]),
                max: corner(&a.local_bounds[1]),
            }),
            resource_name: String::new(),
            reason: None,
            shell_bvh: String::new(),
            instances: Vec::new(),
            notes: Vec::new(),
        }
    }
}

impl PrefabDescriptor {
    /// `None` when any index is outside the library. That is deliberately all-or-nothing: dropping the one unresolvable entry would place a prefab's geometry minus a part of itself, which reads as a correct occluder with a hole in it.
    #[must_use]
    pub fn archived_blas_paths(
        row: &ArchivedOccluderDescriptor,
        index: &[ArchivedBlasEntry],
    ) -> Option<Vec<String>> {
        row.blas
            .iter()
            .map(|i| {
                index
                    .get(i.to_native() as usize)
                    .map(|e| e.path.to_string())
            })
            .collect()
    }
}

impl BlasEntry {
    /// This manifest row as an archive library entry. Lossless — the two shapes are identical.
    #[must_use]
    pub fn to_archived(&self) -> WireBlasEntry {
        WireBlasEntry {
            path: self.path.clone(),
            bytes: self.bytes,
            tris: self.tris,
            kinds: self.kinds,
        }
    }
}

impl BlasEntry {
    /// Read back from the archive's library index. Lossless, so equal to the JSON parse.
    #[must_use]
    pub fn from_archived(a: &ArchivedBlasEntry) -> Self {
        Self {
            path: a.path.to_string(),
            bytes: a.bytes.to_native(),
            tris: a.tris.to_native(),
            kinds: [
                a.kinds[0].to_native(),
                a.kinds[1].to_native(),
                a.kinds[2].to_native(),
            ],
        }
    }
}
