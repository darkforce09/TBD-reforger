//! **Role:** `cargo xtask map bvh-batch --all-prefabs` — the prefab BLAS library.
//!
//! Walks EVERY prefab of `objects/prefabs.json.gz` through the [`Walker`] straight out
//! of the game paks and emits, under `assets/terrains/<terrain>/prefabs/`:
//!
//! - `blas/<stem>.bvh` — one sidecar per distinct XOB (dedup by stem via the walker's asset
//!   cache), shared across every prefab that uses the model;
//! - `descriptors/<pid>.json` — one [`PrefabDescriptor`] per catalogue prefab: the root mesh
//!   as an instance record at identity (kind `Shell` for buildings, the walker's own kind for
//!   everything else) plus every collision-bearing child (doors, frames, panes, furniture), or
//!   `blocks: false` + a reason when nothing in the closure collides;
//! - `blas-manifest.json` — the [`BlasManifest`]: every BLAS with its bytes / tris / kinds, every
//!   descriptor, the per-kind census and the `hot` prefetch set (the most-placed blocking pids).
//!
//! Trees: the trunk and the foliage colliders come from the COLL records exactly as `bvh-batch`
//! reads them (`kind_for_gamemat` / `kind_for_layer`). A tree whose COLL carries no Foliage
//! triangle gets a canopy from the convex hull of its visual LOD0 (`hull_triangles`, the
//! `TreeCanopy` fallback) as an all-Foliage sidecar `blas/<stem>_canopy.bvh`; the census says how
//! many needed it.
//!
//! Deterministic by construction: descriptors and the manifest are sorted, timestamp-free,
//! pretty-printed with a trailing newline, and written through `write_if_changed`, so a re-emit
//! that changes nothing writes nothing.
//!
//! Usage: `--all-prefabs [--terrain everon] [--only-kind tree]… [--limit N] [--hot 100] [--dry-run]
//!         [--paks <dir>] [--extract <dir>] [--out <dir>]`
//! A filtered run (`--only-kind` / `--limit`) writes descriptors + BLAS only — never a partial
//! manifest over a full one.
//!
//! **Position:** called by `bvh-batch --all-prefabs` through [`crate::archive_emission::archive_command`]; its output feeds [`crate::archive_emission::archive_writer`].
//! **Signals & state:** none; reads the game paks and writes the library once per run.
//! **Invariants:** a sidecar or descriptor is written only when its bytes change; the manifest passes its schema before it is written.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt, bail};
use building_interiors::compound::assembly::CoverTier;
use building_interiors::compound::assembly::PlacementSource;
use building_interiors::compound::instances::InstanceKind;
use building_interiors::compound::instances::InstanceRecord;
use building_interiors::compound::instances::LocalTransform;
use geometry_primitives::axis_aligned_box::Bounds3;
use geometry_primitives::rigid_transform::Rigid;
use serde_json::Value;
use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;
use spatial_indexes::bounding_volume_hierarchy::sidecar::emit_bytes;
use spatial_indexes::bounding_volume_hierarchy::sidecar::lift_verts;
use spatial_indexes::bounding_volume_hierarchy::sidecar::quantize_verts;
use spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind;
use spatial_indexes::bounding_volume_hierarchy::triangle_tree::Bvh;
use world_line_of_sight::occluder_library::BlasEntry;
use world_line_of_sight::occluder_library::BlasManifest;
use world_line_of_sight::occluder_library::DESCRIPTOR_SCHEMA_VERSION;
use world_line_of_sight::occluder_library::DescEntry;
use world_line_of_sight::occluder_library::MANIFEST_SCHEMA_VERSION;
use world_line_of_sight::occluder_library::PrefabDescriptor;
use world_line_of_sight::occluder_library::Totals;

use crate::architectural_analysis::convex_hulls::hull_triangles;
use crate::bvh::batch_processing::{
    Asset, Walker, classify_prefab, cover_for_prefab, slug_of, write_if_changed,
};
use crate::bvh::prefab_catalog::strip_guid;
use crate::bvh::world_instances::load_rows;
use crate::mesh_decoding::mesh_format;
use enfusion_pak::AssetSource;

/// Default size of the `hot` prefetch set.
pub(crate) const DEFAULT_HOT: usize = 100;

/// One catalogue row of `objects/prefabs.json.gz`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PrefabRow {
    pub pid: u32,
    /// As the catalogue carries it (`{GUID}Prefabs/…/X.et`).
    pub resource_name: String,
    pub kind: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LibraryOptions {
    pub terrain: String,
    pub only_kinds: Vec<String>,
    pub limit: Option<usize>,
    pub hot: usize,
    /// Which COLL records reach the BLAS (default: the projectile presets).
    pub layer_policy: crate::bvh::batch_processing::LayerPolicy,
}

/// The emitted library, in memory.
pub(crate) struct Library {
    pub descriptors: Vec<PrefabDescriptor>,
    /// `blas/<stem>.bvh` → sidecar bytes.
    pub blas: BTreeMap<String, Vec<u8>>,
    pub manifest: BlasManifest,
}

#[cfg(test)]
#[path = "tests/library_reader_tests.rs"]
mod tests;

#[path = "library_reader/gunzip_json.rs"]
mod gunzip_json;
pub(crate) use gunzip_json::build_library;
pub(crate) use gunzip_json::load_prefab_rows;
pub(crate) use gunzip_json::world_census;

#[path = "library_reader/assemble_manifest.rs"]
mod assemble_manifest;
use assemble_manifest::assemble_manifest;
pub(crate) use assemble_manifest::write_library;

#[cfg(test)]
pub(crate) use assemble_manifest::validate_against;

#[cfg(test)]
pub(crate) use gunzip_json::hull_sample;
