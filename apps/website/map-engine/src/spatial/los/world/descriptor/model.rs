//! Role: model.
//! Position: `spatial/los/world/descriptor` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::Bounds3;
use super::Deserialize;
use super::InstanceRecord;
use super::Serialize;

/// One catalogue prefab's collision closure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefabDescriptor {
    /// Schema version.
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

    /// Notes.
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
