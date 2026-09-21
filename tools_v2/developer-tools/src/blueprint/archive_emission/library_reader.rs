//! `cargo xtask map bvh-batch --all-prefabs` — the prefab BLAS library.
//!
//! Walks EVERY prefab of `objects/prefabs.json.gz` through the [`Walker`] straight out
//! of the game paks and emits, under `assets_v2/terrains/<terrain>/prefabs/`:
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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;
use website_map_engine::spatial::bvh::sidecar::BvhSidecar;
use website_map_engine::spatial::bvh::sidecar::emit_bytes;
use website_map_engine::spatial::bvh::sidecar::lift_verts;
use website_map_engine::spatial::bvh::sidecar::quantize_verts;
use website_map_engine::spatial::bvh::surface::SurfaceKind;
use website_map_engine::spatial::bvh::traversal::Bvh;
use website_map_engine::spatial::los::world::descriptor::BlasEntry;
use website_map_engine::spatial::los::world::descriptor::BlasManifest;
use website_map_engine::spatial::los::world::descriptor::Bounds3;
use website_map_engine::spatial::los::world::descriptor::DESCRIPTOR_SCHEMA_VERSION;
use website_map_engine::spatial::los::world::descriptor::DescEntry;
use website_map_engine::spatial::los::world::descriptor::MANIFEST_SCHEMA_VERSION;
use website_map_engine::spatial::los::world::descriptor::PrefabDescriptor;
use website_map_engine::spatial::los::world::descriptor::Totals;
use website_map_engine::world::architecture::compound::assembly::CoverTier;
use website_map_engine::world::architecture::compound::assembly::PlacementSource;
use website_map_engine::world::architecture::compound::instances::InstanceKind;
use website_map_engine::world::architecture::compound::instances::InstanceRecord;
use website_map_engine::world::architecture::compound::instances::LocalTransform;
use website_map_engine::world::architecture::compound::transform::Rigid;

use super::batch::{Asset, Walker, classify_prefab, cover_for_prefab, slug_of, write_if_changed};
use super::hull::hull_triangles;
use super::prefab::strip_guid;
use super::world_row::load_rows;
use super::xob;
use crate::enfusion_pak::AssetSource;

/// Default size of the `hot` prefetch set.
pub const DEFAULT_HOT: usize = 100;

/// One catalogue row of `objects/prefabs.json.gz`.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefabRow {
    pub pid: u32,
    /// As the catalogue carries it (`{GUID}Prefabs/…/X.et`).
    pub resource_name: String,
    pub kind: String,
}

#[derive(Clone, Debug)]
pub struct LibraryOptions {
    pub terrain: String,
    pub only_kinds: Vec<String>,
    pub limit: Option<usize>,
    pub hot: usize,
    /// Which COLL records reach the BLAS (default: the projectile presets).
    pub layer_policy: super::batch::LayerPolicy,
}

/// The emitted library, in memory.
pub struct Library {
    pub descriptors: Vec<PrefabDescriptor>,
    /// `blas/<stem>.bvh` → sidecar bytes.
    pub blas: BTreeMap<String, Vec<u8>>,
    pub manifest: BlasManifest,
}

#[cfg(test)]
#[path = "../tests/library_tests.rs"]
mod tests;

#[path = "library_reader/gunzip_json.rs"]
mod gunzip_json;
pub use gunzip_json::build_library;
pub use gunzip_json::load_prefab_rows;
pub use gunzip_json::world_census;

#[path = "library_reader/assemble_manifest.rs"]
mod assemble_manifest;
use assemble_manifest::assemble_manifest;
pub use assemble_manifest::write_library;

#[cfg(test)]
pub(crate) use assemble_manifest::validate_against;

#[cfg(test)]
pub(crate) use gunzip_json::hull_sample;
