//! One catalogue prefab's collision closure, as `prefabs/descriptors/<pid>.json` holds it.
//!
//! @contract prefab-descriptor.schema.json#
//!
//! **Role:** declares [`PrefabDescriptor`]: the prefab's catalogue id, slug, resource name and
//! kind, whether anything in it collides, whether it is a tree with foliage triangles, the union
//! of its placed bounds and every placed mesh with the root first.
//! **Position:** parsed by the map engine's occluder loader and expanded by the world occluder;
//! written by the developer tools' blueprint tooling.
//! **Signals & state:** none; plain data.
//! **Invariants:** the JSON shape follows `contracts/definitions/prefab-descriptor.schema.json`;
//! the catalogue id serializes as the bare integer it wraps.

use building_interiors::compound::instances::InstanceRecord;
use geometry_primitives::axis_aligned_box::Bounds3;
use serde::{Deserialize, Serialize};
use world_file_formats::ids::PrefabId;

/// One catalogue prefab's collision closure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefabDescriptor {
    /// The descriptor schema version, [`crate::occluder_library::DESCRIPTOR_SCHEMA_VERSION`].
    pub schema_version: String,

    /// The catalogue `prefabId` (`objects/prefabs.json.gz`).
    pub prefab_id: PrefabId,

    /// File stem of the prefab (`FarmHouse_E_1L01_Wood`).
    pub slug: String,

    /// `Prefabs/…/X.et`, GUID stripped.
    pub resource_name: String,

    /// The catalogue kind (`building`, `tree`, `prop`, …).
    pub kind: String,

    /// Something in the closure collides; `false` descriptors carry no BLAS and never block.
    pub blocks: bool,

    /// Why `blocks` is false: `no-mesh`, `model-unreadable`, `no-coll`, `empty-coll`, `unresolved`.
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

    /// Free-text notes the blueprint tooling left.
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
