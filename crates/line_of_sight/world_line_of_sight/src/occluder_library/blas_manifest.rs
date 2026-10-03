//! The library index `prefabs/blas-manifest.json`: every BLAS file and every descriptor, the hot
//! set and the census.
//!
//! @contract blas-manifest.schema.json#
//!
//! **Role:** declares [`BlasManifest`] with its rows ([`BlasEntry`], [`DescEntry`]) and census
//! ([`Totals`], [`KindTotals`]), the two schema versions, and the binary-search lookups by pid and
//! by path.
//! **Position:** fetched by the map engine's occluder loader; written and checked by the developer
//! tools' blueprint tooling and library checks.
//! **Signals & state:** none; plain data.
//! **Invariants:** the JSON shape follows `contracts/definitions/blas-manifest.schema.json`; BLAS
//! rows are sorted by path and descriptor rows by pid, which the lookups rely on.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use world_file_formats::ids::TerrainId;

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

    /// The file's size in bytes.
    pub bytes: u64,

    /// The file's triangle count.
    pub tris: u32,

    /// Triangle counts per `SurfaceKind`: `[opaque, glass, foliage]`.
    pub kinds: [u32; 3],
}

/// One descriptor in the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescEntry {
    /// The catalogue prefab the descriptor describes.
    pub pid: u32,

    /// `descriptors/<pid>.json`, relative to the prefabs root.
    pub path: String,

    /// The catalogue kind.
    pub kind: String,

    /// Whether anything in the prefab collides.
    pub blocks: bool,

    /// Whether the prefab is a tree with foliage triangles.
    pub canopy: bool,

    /// Distinct BLAS paths the descriptor references.
    pub blas: Vec<String>,

    /// The number of placed meshes.
    pub instance_count: u32,

    /// How many chunk rows place this prefab on the terrain (the hot-set key).
    pub instances_in_world: u64,
}

/// Per-kind census of the library.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KindTotals {
    /// Descriptors of this kind (JSON `prefabs`).
    pub prefabs: u32,

    /// Descriptors of this kind with `blocks: true` (JSON `blocks`).
    pub blocks: u32,

    /// Non-blocking descriptors whose prefab resolves to no mesh (reason `no-mesh`).
    pub no_mesh: u32,

    /// Non-blocking descriptors whose mesh resolves but whose model does not load (reason
    /// `model-unreadable`).
    pub model_unreadable: u32,

    /// Non-blocking descriptors whose model carries no collision mesh (reason `no-coll`).
    pub no_coll: u32,

    /// Non-blocking descriptors whose collision mesh has no triangles (reason `empty-coll`).
    pub empty_coll: u32,

    /// Non-blocking descriptors with reason `unresolved` or any reason not counted above.
    pub unresolved: u32,

    /// Non-blocking descriptors whose collision triangles all sit on presets projectiles pass
    /// through (reason `no-fire-geo`); 0 when the JSON omits `noFireGeo`.
    #[serde(default)]
    pub no_fire_geo: u32,

    /// Bytes of the BLAS files first referenced by this kind's descriptors.
    pub bytes: u64,
}

/// Library-wide census.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    /// Descriptors in the library (JSON `prefabs`).
    pub prefabs: u32,

    /// Descriptors with `blocks: true`, which carry BLAS and occlude sight lines.
    pub blocks: u32,

    /// Descriptors with `canopy: true`: trees whose BLAS carries foliage triangles.
    pub canopy: u32,

    /// Trees whose canopy is the visual-LOD hull fallback.
    pub canopy_hull: u32,

    /// Number of BLAS files in the library (JSON `blasFiles`).
    pub blas_files: u32,

    /// Summed size in bytes of every BLAS file in the library (JSON `blasBytes`).
    pub blas_bytes: u64,

    /// The per-kind census keyed by catalogue kind (`building`, `tree`, `prop`, ...), sorted by
    /// kind (JSON `byKind`).
    pub by_kind: BTreeMap<String, KindTotals>,
}

/// `prefabs/blas-manifest.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlasManifest {
    /// Contract version of the manifest file, [`MANIFEST_SCHEMA_VERSION`] when written.
    pub schema_version: String,

    /// The terrain the library belongs to, such as `everon`.
    pub terrain_id: TerrainId,

    /// Sorted by path.
    pub blas: Vec<BlasEntry>,

    /// Sorted by pid.
    pub descriptors: Vec<DescEntry>,

    /// The prefabs the SPA prefetches at boot: the most-placed `blocks: true` pids, most first.
    pub hot: Vec<u32>,

    /// The library-wide census, with its per-kind breakdown.
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
