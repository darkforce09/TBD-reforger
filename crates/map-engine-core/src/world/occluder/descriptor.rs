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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::building_compound::InstanceRecord;

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
