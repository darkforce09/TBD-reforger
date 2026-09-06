//! T-090.12.2 — the prefab BLAS library wire types.
//!
//! One [`PrefabDescriptor`] per catalogue prefab (`prefabs/descriptors/<pid>.json`): the root
//! collision mesh and every child the prefab slots in (doors, frames, panes, furniture — the
//! T-090.11.2 walker) as placed BLAS instance records in the prefab's own frame, or an explicit
//! `blocks: false` with a reason when nothing in the closure collides — a decal, a light, a sound
//! source never phantom-blocks a shot. One [`BlasManifest`] (`prefabs/blas-manifest.json`) indexes
//! the library: every BLAS file with its byte size, every descriptor, and the `hot` set the SPA
//! prefetches at boot. Both are deterministic — sorted, timestamp-free — so a re-emit that changes
//! nothing writes nothing.
//!
//! Frames: instance transforms are the Enfusion object frame (`x`, `y` up, `z`; metres), exactly
//! as `<slug>.instances.json` — a descriptor of a building IS its instances file plus the root
//! record. The root record's `kind` is [`InstanceKind::Shell`](crate::building_compound::InstanceKind)
//! for buildings and the walker's own classification (`Tree`, `Prop`, …) for everything else, so a
//! hit on it reads as what it is.
//!
//! ── T-935.8: the archive side ────────────────────────────────────────────────────────────────
//! [`BuildingArchiveBytes`] holds `prefabs/building_blueprints.rkyv` and
//! [`PrefabDescriptor::from_archived`] reads one row of it, so a loader gets the whole library's
//! census — which pids block, which carry canopy, their bounds, and every pid's `.bvh` fetch list
//! ([`PrefabDescriptor::archived_blas_paths`]) — from ONE fetch instead of one JSON per prefab.
//!
//! **The archive is a census, not a replacement.** Its row
//! ([`archives::OccluderDescriptor`](crate::world::binary::archives::OccluderDescriptor)) carries
//! no instance records, so a rebuilt descriptor has no placed geometry. Handing one to
//! [`WorldOccluder::insert_descriptor`](super::WorldOccluder::insert_descriptor) for a
//! `blocks: true` prefab registers the pid as known-and-expandable and then expands it to nothing:
//! the prefab stops occluding, forever, with no error anywhere. That is why
//! [`PrefabDescriptor::archive_census`] spells out the loss and why the occluder host's archive
//! branch inserts rebuilt descriptors only where `blocks` is false.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::building_compound::InstanceRecord;
use crate::world::binary::archives::{
    ARCHIVE_SCHEMA_VERSION, ArchivedBlasEntry, ArchivedBuildingBlueprintArchive,
    ArchivedOccluderDescriptor, BlasEntry as WireBlasEntry, BuildingBlueprintArchive,
    OccluderDescriptor as WireDescriptor,
};
use crate::world::binary::{BinaryError, access_checked};

/// Contract version of `prefabs/descriptors/<pid>.json`.
pub const DESCRIPTOR_SCHEMA_VERSION: &str = "1.0.0";
/// Contract version of `prefabs/blas-manifest.json`.
pub const MANIFEST_SCHEMA_VERSION: &str = "1.0.0";

/// An axis-aligned box in the prefab's object frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds3 {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Bounds3 {
    /// The union of two boxes.
    #[must_use]
    pub fn union(self, o: Bounds3) -> Bounds3 {
        let mut b = self;
        for a in 0..3 {
            b.min[a] = b.min[a].min(o.min[a]);
            b.max[a] = b.max[a].max(o.max[a]);
        }
        b
    }

    /// This box as `prefabs/building_blueprints.rkyv` stores it — every coordinate through `f32`
    /// and back. T-935.8: the archive is the `f32` wire twin (archives.rs module docs), so a
    /// JSON-parsed descriptor and its archived row can only agree to `f32`, and a parity test that
    /// compares them must say so in one place rather than sprinkle tolerances.
    #[must_use]
    pub fn to_f32_lossy(self) -> Bounds3 {
        Bounds3 {
            min: self.min.map(f32_round),
            max: self.max.map(f32_round),
        }
    }
}

/// One coordinate through the archive's `f32` and back.
fn f32_round(v: f64) -> f64 {
    f64::from(v as f32)
}

/// One catalogue prefab's collision closure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefabDescriptor {
    pub schema_version: String,
    /// The catalogue `prefabId` (`objects/prefabs.json.gz`).
    pub prefab_id: u32,
    /// File stem of the prefab (`FarmHouse_E_1L01_Wood`).
    pub slug: String,
    /// `Prefabs/…/X.et`, GUID stripped.
    pub resource_name: String,
    /// The catalogue kind (`building`, `tree`, `prop`, …).
    pub kind: String,
    /// Something in the closure collides; `false` descriptors carry no BLAS and never block.
    pub blocks: bool,
    /// Why `blocks` is false: `no-mesh`, `model-unreadable`, `no-coll`, `empty-coll`,
    /// `unresolved`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// A tree whose BLAS carries Foliage triangles (from its COLL, or the hull fallback).
    pub canopy: bool,
    /// Union of every instance's placed bounds, object frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_bounds: Option<Bounds3>,
    /// The root record's BLAS path (`blas/<stem>.bvh`), empty when the root has no collision.
    pub shell_bvh: String,
    /// Every placed BLAS, root first; paths relative to the prefabs root.
    pub instances: Vec<InstanceRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl PrefabDescriptor {
    /// Every distinct BLAS path referenced, in first-use order.
    #[must_use]
    pub fn blas_paths(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for i in &self.instances {
            if !out.contains(&i.blas.as_str()) {
                out.push(&i.blas);
            }
        }
        out
    }

    /// T-935.8 — this descriptor as a `prefabs/building_blueprints.rkyv` row, with every BLAS path
    /// resolved to its index in the archive's shared library through `index_of`.
    ///
    /// # Errors
    /// [`ArchiveProjectionError`] — a BLAS the descriptor names is not in the library, or `blocks`
    /// and `localBounds` disagree. Both are refusals rather than best-effort writes: the archive's
    /// `local_bounds` is *not* optional, so a blocking-but-unbounded descriptor would be written as
    /// a zero-size box, and a prefab that occludes nothing while reporting no error is the
    /// silently-wrong-sightline failure this archive exists to avoid.
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

    /// The descriptor reduced to exactly what the archive can carry — and the cleared fields ARE
    /// the documented loss, in code rather than in prose.
    ///
    /// `resource_name`, `reason`, `notes` are diagnostics no occluder reads. **`shell_bvh` and
    /// `instances` are not**: `instances` carries every child's placed transform, and 235 of the
    /// 1623 committed descriptors have more than one. A descriptor rebuilt from the archive
    /// therefore has an EMPTY instance list, and
    /// [`WorldOccluder::insert_descriptor`](super::WorldOccluder::insert_descriptor) expands an
    /// empty list to no geometry — so [`from_archived`](Self::from_archived) must never be handed
    /// to it for a `blocks: true` prefab. See the module note on the archive branch.
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

    /// T-935.8 — rebuild a descriptor from its archived row. Equal to
    /// [`archive_census`](Self::archive_census) of the descriptor the row was projected from, for
    /// every one of the 1623 committed descriptors (`archive_emit` parity test).
    ///
    /// Read [`archive_census`](Self::archive_census) before using this: the value has NO instance
    /// records and must not reach `insert_descriptor` unless `blocks` is false.
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

    /// T-935.8 — the `.bvh` fetch list of an archived row: its `blas` indices resolved through the
    /// archive's shared library, first-use order, root first. The archive twin of
    /// [`blas_paths`](Self::blas_paths), and the whole point of `blas_index` — this is what lets
    /// the occluder host queue a prefab's sidecars without its descriptor JSON.
    ///
    /// `None` when any index is outside the library. That is deliberately all-or-nothing: dropping
    /// the one unresolvable entry would place a prefab's geometry minus a part of itself, which
    /// reads as a correct occluder with a hole in it.
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

/// Why a descriptor cannot be written into `prefabs/building_blueprints.rkyv` (T-935.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchiveProjectionError {
    /// A BLAS path the descriptor names is absent from the library index the archive carries.
    UnknownBlas { prefab_id: u32, path: String },
    /// `blocks` and `localBounds` disagree. All 1623 committed descriptors satisfy
    /// `blocks == localBounds.is_some()`; the archive's non-optional `local_bounds` relies on it.
    BoundsDisagreeWithBlocks { prefab_id: u32, blocks: bool },
}

impl fmt::Display for ArchiveProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// T-935.8 — `prefabs/building_blueprints.rkyv` as a loader holds it: the fetched bytes copied
/// once into an 8-aligned buffer, then read in place.
///
/// The copy is not incidental. [`access_checked`] *validates* alignment rather than assuming it,
/// and the SPA's `fetch_bytes` hands back a `Vec<u8>` whose allocation is 1-aligned — so accessing
/// the fetched buffer directly is a coin flip that reads as "the archive is corrupt" when it loses.
/// Backing the copy with a `Vec<u64>` gets 8-byte alignment from the allocator by construction
/// (`BlasEntry::bytes` is a `u64`, the widest scalar in the archive) with no `unsafe` and no new
/// dependency in the SPA.
pub struct BuildingArchiveBytes {
    words: Vec<u64>,
    len: usize,
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

    /// The archive bytes, 8-aligned.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &bytemuck::cast_slice(&self.words)[..self.len]
    }

    /// The validated archive, borrowed in place — no deserialise, no allocation.
    ///
    /// VALIDATION IS LAYOUT PLUS MEANING. `access_checked` proves rkyv can read the buffer; it
    /// says nothing about whether this build agrees with the writer about what the fields MEAN.
    /// T-946: the wave 238 verifier found this accessor taking a future archive on layout alone,
    /// while its sibling `world::locations::map_labels_from_bytes` — written in the same wave —
    /// checked the version. For labels a mismatch draws the wrong text. Here it seeds `no_block`
    /// for the wrong prefab ids, `wanted()` never asks for them again, and the building stops
    /// occluding for the whole session with nothing logged: the silently wrong sightline this
    /// module exists to prevent.
    ///
    /// # Errors
    /// [`BinaryError::Archive`] when the buffer fails rkyv validation;
    /// [`BinaryError::UnsupportedVersion`] when it was written to a schema this build does not
    /// implement. Both are the caller's cue to fall back to the descriptor JSON.
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

/// T-935.8 — what one `prefabs/building_blueprints.rkyv` fetch tells an occluder loader, split
/// into the half it may hand straight to
/// [`WorldOccluder::insert_descriptor`](super::WorldOccluder::insert_descriptor) and the half it
/// may not.
///
/// The split is the whole point, and it is here rather than in the SPA so it is testable natively
/// against the real corpus: the archive row has no [`InstanceRecord`]s, so a `blocks: true`
/// descriptor rebuilt from it would enter `WorldOccluder::descriptors`, fail `try_expand`
/// (`instances.is_empty()`), and then be skipped forever by `wanted()` — which only asks for a
/// descriptor it does not already hold. The prefab would trace against its coarse AABB for the
/// rest of the session with no error anywhere. That is the silently-wrong sightline this archive
/// exists to avoid, so [`census`](Self::census) carries `blocks: false` rows only.
///
/// [`blas_by_pid`](Self::blas_by_pid) is the half that *is* complete for every prefab: the `.bvh`
/// fetch list of every row, resolved through the archive's shared library. It is what lets a
/// loader queue a prefab's sidecars in the same round as its descriptor instead of the round
/// after.
pub struct ArchiveBoot {
    /// Rebuilt `blocks: false` descriptors — safe to insert, and the reason those pids are never
    /// fetched again (`insert_descriptor` routes them to `no_block`).
    pub census: Vec<PrefabDescriptor>,
    /// `(pid, .bvh paths)` for **every** archived row, pid-ascending, paths in first-use order.
    pub blas_by_pid: Vec<(u16, Vec<String>)>,
    /// Archived rows with `blocks: true` — the descriptors a loader must still read as JSON.
    pub blocking: usize,
    /// Rows whose `prefab_id` does not fit `u16`, or whose BLAS list does not resolve against
    /// `blas_index`. Both are dropped rather than guessed, and counted so a loader can say so.
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

/// One row of the archive's shared BLAS library, back as the manifest's own [`BlasEntry`].
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

/// One BLAS file in the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlasEntry {
    /// `blas/<stem>.bvh`, relative to the prefabs root.
    pub path: String,
    pub bytes: u64,
    pub tris: u32,
    /// Triangle counts per `SurfaceKind`: `[opaque, glass, foliage]`.
    pub kinds: [u32; 3],
}

/// One descriptor in the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescEntry {
    pub pid: u32,
    /// `descriptors/<pid>.json`, relative to the prefabs root.
    pub path: String,
    pub kind: String,
    pub blocks: bool,
    pub canopy: bool,
    /// Distinct BLAS paths the descriptor references.
    pub blas: Vec<String>,
    pub instance_count: u32,
    /// How many chunk rows place this prefab on the terrain (the hot-set key).
    pub instances_in_world: u64,
}

/// Per-kind census of the library.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KindTotals {
    pub prefabs: u32,
    pub blocks: u32,
    pub no_mesh: u32,
    pub model_unreadable: u32,
    pub no_coll: u32,
    pub empty_coll: u32,
    pub unresolved: u32,
    /// Collision exists but every record sits on a preset projectiles pass through
    /// (T-090.12.4 layer policy).
    #[serde(default)]
    pub no_fire_geo: u32,
    /// Bytes of the BLAS files first referenced by this kind's descriptors.
    pub bytes: u64,
}

/// Library-wide census.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub prefabs: u32,
    pub blocks: u32,
    pub canopy: u32,
    /// Trees whose canopy is the visual-LOD hull fallback.
    pub canopy_hull: u32,
    pub blas_files: u32,
    pub blas_bytes: u64,
    pub by_kind: BTreeMap<String, KindTotals>,
}

/// `prefabs/blas-manifest.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlasManifest {
    pub schema_version: String,
    pub terrain_id: String,
    /// Sorted by path.
    pub blas: Vec<BlasEntry>,
    /// Sorted by pid.
    pub descriptors: Vec<DescEntry>,
    /// The prefabs the SPA prefetches at boot: the most-placed `blocks: true` pids, most first.
    pub hot: Vec<u32>,
    pub totals: Totals,
}

impl BlasManifest {
    /// The descriptor entry of `pid`.
    #[must_use]
    pub fn descriptor(&self, pid: u32) -> Option<&DescEntry> {
        self.descriptors
            .binary_search_by_key(&pid, |d| d.pid)
            .ok()
            .map(|i| &self.descriptors[i])
    }
    /// The BLAS entry at `path`.
    #[must_use]
    pub fn blas(&self, path: &str) -> Option<&BlasEntry> {
        self.blas
            .binary_search_by(|b| b.path.as_str().cmp(path))
            .ok()
            .map(|i| &self.blas[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building_compound::{CoverTier, InstanceKind, LocalTransform, PlacementSource};

    fn record(id: &str, kind: InstanceKind, blas: &str) -> InstanceRecord {
        InstanceRecord {
            id: id.into(),
            kind,
            prefab: "Prefabs/X.et".into(),
            blas: blas.into(),
            xob: None,
            local: LocalTransform::identity(),
            door: None,
            cover: CoverTier::None,
            source: PlacementSource::PrefabCoords,
            parent: None,
        }
    }

    #[test]
    fn descriptor_round_trips_and_lists_distinct_blas_in_first_use_order() {
        let d = PrefabDescriptor {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
            prefab_id: 7,
            slug: "X".into(),
            resource_name: "Prefabs/X.et".into(),
            kind: "building".into(),
            blocks: true,
            reason: None,
            canopy: false,
            local_bounds: Some(Bounds3 {
                min: [-1.0, 0.0, -2.0],
                max: [1.0, 3.0, 2.0],
            }),
            shell_bvh: "blas/x.bvh".into(),
            instances: vec![
                record("X", InstanceKind::Shell, "blas/x.bvh"),
                record("X/door", InstanceKind::DoorLeaf, "blas/door.bvh"),
                record("X/door2", InstanceKind::DoorLeaf, "blas/door.bvh"),
            ],
            notes: vec![],
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("\"schemaVersion\":\"1.0.0\""));
        assert!(json.contains("\"prefabId\":7"));
        assert!(!json.contains("\"reason\""), "None reason is omitted");
        assert!(!json.contains("\"notes\""), "empty notes are omitted");
        let back: PrefabDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
        assert_eq!(back.blas_paths(), vec!["blas/x.bvh", "blas/door.bvh"]);
    }

    #[test]
    fn blocks_false_descriptor_carries_its_reason_and_no_blas() {
        let d = PrefabDescriptor {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
            prefab_id: 9,
            slug: "Decal".into(),
            resource_name: "Prefabs/Decal.et".into(),
            kind: "prop".into(),
            blocks: false,
            reason: Some("no-coll".into()),
            canopy: false,
            local_bounds: None,
            shell_bvh: String::new(),
            instances: vec![],
            notes: vec!["Decal: Assets/decal.xob has no collision chunk".into()],
        };
        let back: PrefabDescriptor =
            serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(back, d);
        assert!(back.blas_paths().is_empty());
    }

    #[test]
    fn manifest_lookups_are_binary_searches_over_sorted_entries() {
        let m = BlasManifest {
            schema_version: MANIFEST_SCHEMA_VERSION.into(),
            terrain_id: "everon".into(),
            blas: vec![
                BlasEntry {
                    path: "blas/a.bvh".into(),
                    bytes: 10,
                    tris: 1,
                    kinds: [1, 0, 0],
                },
                BlasEntry {
                    path: "blas/b.bvh".into(),
                    bytes: 20,
                    tris: 2,
                    kinds: [0, 0, 2],
                },
            ],
            descriptors: vec![
                DescEntry {
                    pid: 3,
                    path: "descriptors/3.json".into(),
                    kind: "tree".into(),
                    blocks: true,
                    canopy: true,
                    blas: vec!["blas/b.bvh".into()],
                    instance_count: 1,
                    instances_in_world: 500,
                },
                DescEntry {
                    pid: 8,
                    path: "descriptors/8.json".into(),
                    kind: "prop".into(),
                    blocks: false,
                    canopy: false,
                    blas: vec![],
                    instance_count: 0,
                    instances_in_world: 2,
                },
            ],
            hot: vec![3],
            totals: Totals::default(),
        };
        assert_eq!(m.descriptor(3).map(|d| d.instances_in_world), Some(500));
        assert_eq!(m.descriptor(4), None);
        assert_eq!(m.blas("blas/b.bvh").map(|b| b.tris), Some(2));
        assert_eq!(m.blas("blas/c.bvh"), None);
        let back: BlasManifest = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back, m);
    }

    /* ─────────────────── T-935.8 — the building_blueprints.rkyv row ─────────────────── */

    use crate::world::binary::archives::{ARCHIVE_SCHEMA_VERSION, BuildingBlueprintArchive};
    use crate::world::binary::to_bytes;

    fn library() -> Vec<BlasEntry> {
        vec![
            BlasEntry {
                path: "blas/x.bvh".into(),
                bytes: 10,
                tris: 1,
                kinds: [1, 0, 0],
            },
            BlasEntry {
                path: "blas/door.bvh".into(),
                bytes: 20,
                tris: 2,
                kinds: [2, 0, 0],
            },
        ]
    }

    fn index_of(lib: &[BlasEntry]) -> impl Fn(&str) -> Option<u32> + '_ {
        move |p: &str| {
            lib.iter()
                .position(|e| e.path == p)
                .and_then(|i| u32::try_from(i).ok())
        }
    }

    /// A one-descriptor archive, serialised and read back through the real validating reader —
    /// so these tests exercise the same path a loader does, not an in-memory struct.
    fn archived(d: &PrefabDescriptor, lib: &[BlasEntry]) -> BuildingArchiveBytes {
        let a = BuildingBlueprintArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            descriptors: vec![d.to_archived(&index_of(lib)).expect("project")],
            blas_index: lib.iter().map(BlasEntry::to_archived).collect(),
            blueprints: vec![],
        };
        BuildingArchiveBytes::new(&to_bytes(&a).expect("serialise"))
    }

    /// T-946 — a future archive must be refused, not read on layout alone.
    ///
    /// rkyv's `access_checked` proves the buffer is READABLE, not that this build agrees with the
    /// writer about what the fields mean. Found by the wave 238 verifier: a v2 archive whose
    /// `blocks` bits had shifted would seed `no_block` for the wrong prefab ids, `wanted()` would
    /// never ask for them again, and those buildings would stop occluding for the whole session
    /// with nothing logged. The sibling label archive checked its version in the same wave.
    #[test]
    fn a_future_archive_schema_is_refused_rather_than_read() {
        let future = BuildingBlueprintArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION + 1,
            descriptors: vec![],
            blas_index: vec![],
            blueprints: vec![],
        };
        let bytes = BuildingArchiveBytes::new(&to_bytes(&future).expect("serialise"));
        let err = bytes
            .archive()
            .expect_err("a schema this build does not implement must not be read");
        println!("── refusal ── {err}");
        assert!(
            matches!(err, BinaryError::UnsupportedVersion { .. }),
            "and it must say so, not blame the layout: {err:?}"
        );
        // The current version still reads.
        let current = BuildingBlueprintArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            descriptors: vec![],
            blas_index: vec![],
            blueprints: vec![],
        };
        let ok = BuildingArchiveBytes::new(&to_bytes(&current).expect("serialise"));
        assert!(ok.archive().is_ok());
    }

    #[test]
    fn archived_descriptor_round_trips_to_its_census_and_keeps_the_blas_order() {
        let d = PrefabDescriptor {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
            prefab_id: 7,
            slug: "X".into(),
            resource_name: "Prefabs/X.et".into(),
            kind: "building".into(),
            blocks: true,
            reason: None,
            canopy: false,
            local_bounds: Some(Bounds3 {
                min: [-1.5, 0.0, -2.25],
                max: [1.5, 3.0, 2.25],
            }),
            shell_bvh: "blas/x.bvh".into(),
            instances: vec![
                record("X", InstanceKind::Shell, "blas/x.bvh"),
                record("X/door", InstanceKind::DoorLeaf, "blas/door.bvh"),
                record("X/door2", InstanceKind::DoorLeaf, "blas/door.bvh"),
            ],
            notes: vec![],
        };
        let lib = library();
        let held = archived(&d, &lib);
        let a = held.archive().expect("access_checked validates");
        let row = &a.descriptors[0];

        assert_eq!(PrefabDescriptor::from_archived(row), d.archive_census());
        // The oracle is the JSON descriptor's own path list, resolved through the index — not the
        // census, so a lost or reordered index is caught here and not by a symmetric comparison.
        assert_eq!(
            PrefabDescriptor::archived_blas_paths(row, &a.blas_index),
            Some(
                d.blas_paths()
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(
            BlasEntry::from_archived(&a.blas_index[1]),
            lib[1],
            "the library index is lossless"
        );
    }

    #[test]
    fn a_non_blocking_descriptor_archives_with_no_bounds_and_no_blas() {
        let d = PrefabDescriptor {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
            prefab_id: 9,
            slug: "Decal".into(),
            resource_name: "Prefabs/Decal.et".into(),
            kind: "prop".into(),
            blocks: false,
            reason: Some("no-coll".into()),
            canopy: false,
            local_bounds: None,
            shell_bvh: String::new(),
            instances: vec![],
            notes: vec!["Decal: no collision chunk".into()],
        };
        let lib = library();
        let held = archived(&d, &lib);
        let a = held.archive().expect("access");
        let back = PrefabDescriptor::from_archived(&a.descriptors[0]);
        assert_eq!(back, d.archive_census());
        assert!(!back.blocks);
        assert_eq!(back.local_bounds, None, "blocks: false carries no bounds");
        assert_eq!(
            PrefabDescriptor::archived_blas_paths(&a.descriptors[0], &a.blas_index),
            Some(vec![])
        );
    }

    /// The archive's `local_bounds` is not optional, so the projection must refuse the pair it
    /// cannot represent rather than write a zero box that occludes nothing.
    #[test]
    fn projection_refuses_bounds_that_disagree_with_blocks_and_an_unknown_blas() {
        let mut d = PrefabDescriptor {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
            prefab_id: 4,
            slug: "Y".into(),
            resource_name: "Prefabs/Y.et".into(),
            kind: "prop".into(),
            blocks: true,
            reason: None,
            canopy: false,
            local_bounds: None,
            shell_bvh: "blas/x.bvh".into(),
            instances: vec![record("Y", InstanceKind::Prop, "blas/x.bvh")],
            notes: vec![],
        };
        let lib = library();
        assert_eq!(
            d.to_archived(&index_of(&lib)),
            Err(ArchiveProjectionError::BoundsDisagreeWithBlocks {
                prefab_id: 4,
                blocks: true
            })
        );
        d.local_bounds = Some(Bounds3 {
            min: [0.0; 3],
            max: [1.0; 3],
        });
        d.instances = vec![record("Y", InstanceKind::Prop, "blas/missing.bvh")];
        assert_eq!(
            d.to_archived(&index_of(&lib)),
            Err(ArchiveProjectionError::UnknownBlas {
                prefab_id: 4,
                path: "blas/missing.bvh".into()
            })
        );
        assert!(
            !ArchiveProjectionError::UnknownBlas {
                prefab_id: 4,
                path: "blas/missing.bvh".into()
            }
            .to_string()
            .is_empty()
        );
    }

    /// A `Vec<u8>` off `fetch_bytes` is 1-aligned; `access_checked` validates alignment. The
    /// holder must survive being handed a buffer at every byte offset, and hand back the bytes
    /// unchanged — including the non-multiple-of-8 tail.
    #[test]
    fn aligned_holder_preserves_bytes_and_reads_a_misaligned_source() {
        let src: Vec<u8> = (0u8..=250).collect();
        let held = BuildingArchiveBytes::new(&src);
        assert_eq!(held.as_slice(), &src[..], "251 bytes, not a multiple of 8");
        assert_eq!(
            held.as_slice().as_ptr() as usize % 8,
            0,
            "the archive buffer must be 8-aligned"
        );
        let d = PrefabDescriptor {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
            prefab_id: 1,
            slug: "Z".into(),
            resource_name: String::new(),
            kind: "prop".into(),
            blocks: false,
            reason: None,
            canopy: false,
            local_bounds: None,
            shell_bvh: String::new(),
            instances: vec![],
            notes: vec![],
        };
        let lib = library();
        let bytes = archived(&d, &lib).as_slice().to_vec();
        // Offset by one byte in a fresh Vec, exactly as a 1-aligned fetch buffer can land.
        let mut shifted = vec![0u8];
        shifted.extend_from_slice(&bytes);
        let held = BuildingArchiveBytes::new(&shifted[1..]);
        assert_eq!(
            held.archive()
                .expect("re-aligned copy validates")
                .descriptors[0]
                .slug
                .as_str(),
            "Z"
        );
        assert!(
            BuildingArchiveBytes::new(&bytes[..bytes.len() - 4])
                .archive()
                .is_err(),
            "a truncated archive is an error, never a wild read"
        );
    }

    #[test]
    fn bounds_union_is_componentwise() {
        let a = Bounds3 {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        };
        let b = Bounds3 {
            min: [-1.0, 0.5, 0.0],
            max: [0.5, 2.0, 3.0],
        };
        assert_eq!(
            a.union(b),
            Bounds3 {
                min: [-1.0, 0.0, 0.0],
                max: [1.0, 2.0, 3.0]
            }
        );
    }
}
