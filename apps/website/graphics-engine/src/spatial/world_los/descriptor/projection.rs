//! Role: projection.
//! Position: `spatial/world_los/descriptor` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::ArchiveProjectionError;
use super::ArchivedBlasEntry;
use super::ArchivedOccluderDescriptor;
use super::BlasEntry;
use super::Bounds3;
use super::DESCRIPTOR_SCHEMA_VERSION;
use super::PrefabDescriptor;
use super::WireBlasEntry;
use super::WireDescriptor;

impl PrefabDescriptor {
    /// To archived.
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
    /// From archived.
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
