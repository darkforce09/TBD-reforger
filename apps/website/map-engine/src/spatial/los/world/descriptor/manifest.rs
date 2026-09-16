//! Role: manifest.
//! Position: `spatial/los/world/descriptor` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::BTreeMap;
use super::Deserialize;
use super::Serialize;

/// Contract version of `prefabs/descriptors/<pid>.json`.
pub const DESCRIPTOR_SCHEMA_VERSION: &str = "1.0.0";

/// Contract version of `prefabs/blas-manifest.json`.
pub const MANIFEST_SCHEMA_VERSION: &str = "1.0.0";

/// One BLAS file in the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlasEntry {
    /// `blas/<stem>.bvh`, relative to the prefabs root.
    pub path: String,

    /// Bytes.
    pub bytes: u64,

    /// Tris.
    pub tris: u32,

    /// Triangle counts per `SurfaceKind`: `[opaque, glass, foliage]`.
    pub kinds: [u32; 3],
}

/// One descriptor in the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescEntry {
    /// Pid.
    pub pid: u32,

    /// `descriptors/<pid>.json`, relative to the prefabs root.
    pub path: String,

    /// Kind.
    pub kind: String,

    /// Blocks.
    pub blocks: bool,

    /// Canopy.
    pub canopy: bool,

    /// Distinct BLAS paths the descriptor references.
    pub blas: Vec<String>,

    /// Instance count.
    pub instance_count: u32,

    /// How many chunk rows place this prefab on the terrain (the hot-set key).
    pub instances_in_world: u64,
}

/// Per-kind census of the library.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KindTotals {
    /// Prefabs.
    pub prefabs: u32,

    /// Blocks.
    pub blocks: u32,

    /// No mesh.
    pub no_mesh: u32,

    /// Model unreadable.
    pub model_unreadable: u32,

    /// No coll.
    pub no_coll: u32,

    /// Empty coll.
    pub empty_coll: u32,

    /// Unresolved.
    pub unresolved: u32,

    /// No fire geo.
    #[serde(default)]
    pub no_fire_geo: u32,

    /// Bytes of the BLAS files first referenced by this kind's descriptors.
    pub bytes: u64,
}

/// Library-wide census.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    /// Prefabs.
    pub prefabs: u32,

    /// Blocks.
    pub blocks: u32,

    /// Canopy.
    pub canopy: u32,

    /// Trees whose canopy is the visual-LOD hull fallback.
    pub canopy_hull: u32,

    /// Blas files.
    pub blas_files: u32,

    /// Blas bytes.
    pub blas_bytes: u64,

    /// By kind.
    pub by_kind: BTreeMap<String, KindTotals>,
}

/// `prefabs/blas-manifest.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlasManifest {
    /// Schema version.
    pub schema_version: String,

    /// Terrain id.
    pub terrain_id: String,

    /// Sorted by path.
    pub blas: Vec<BlasEntry>,

    /// Sorted by pid.
    pub descriptors: Vec<DescEntry>,

    /// The prefabs the SPA prefetches at boot: the most-placed `blocks: true` pids, most first.
    pub hot: Vec<u32>,

    /// Totals.
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
}

impl BlasManifest {
    /// The BLAS entry at `path`.
    #[must_use]
    pub fn blas(&self, path: &str) -> Option<&BlasEntry> {
        self.blas
            .binary_search_by(|b| b.path.as_str().cmp(path))
            .ok()
            .map(|i| &self.blas[i])
    }
}
