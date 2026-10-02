//! Catalog-v1 world-object build + roads. Content-identical to the documented artifact
//! contract: identical JSON bytes
//! before compression (js_num integral-number semantics, identical key order via preserve_order,
//! identical sorts), gzip level 9 (flate2 — the N5 one-time re-encode swaps the committed gz
//! container bytes; decompressed content is the proven-equal contract).

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde_json::{Map, Value, json};

use super::catalog_emit;
use super::classify::Classifier;
use super::forest_smoothing;
use super::json_number_formatting::{js_num, round2};
use super::topo::{
    TOPO_AIRFIELD, TOPO_FARM_TRACK, TOPO_GRAVEL_COUNTRY_ROAD, TOPO_MAIN_HIGHWAY,
    TOPO_SECONDARY_ASPHALT, decode_topo,
};
use crate::browser_testing::server::repo_root;
use crate::enfusion_pak::PakVfs;
use crate::world_export_pipeline::forest_contours as forest;
use crate::world_export_pipeline::forest_contours::{Tree, derive_forest_regions};
use crate::world_export_pipeline::polygon_geometry::chunk_key;
use crate::world_export_pipeline::vegetation_density as density;

mod object_partitioning;

pub const CHUNK_SIZE_M: f64 = 512.0;

pub const PHASE_ORDER: [&str; 5] = [
    "P1_buildings",
    "P2_trees",
    "P3_vegetation",
    "P4_rocks",
    "P5_props",
];

/// One partitioned chunk row (full transform; trivial trailers are written 5-wide).
struct ChunkRow {
    id: usize,
    x: f64,
    y: f64,
    z: f64,
    rot: f64,
    pitch: f64,
    roll: f64,
    scale: f64,
}

struct KeptRow {
    resource_name: String,
    kind: String,
    x: f64,
    y: f64,
    z: f64,
    rot: f64,
    /// `pitchDeg` / `rollDeg` (round2; every export carries them)
    /// and `scale` (round3; written by the v2 exporter only, else `1.0`).
    pitch: f64,
    roll: f64,
    scale: f64,
}

pub struct BuildSummary {
    pub summary: Value,
}

#[cfg(test)]
#[path = "tests/chunk_partitioner/tests.rs"]
mod tests;

#[path = "chunk_partitioner/may_clear_density_dir.rs"]
mod may_clear_density_dir;
pub use may_clear_density_dir::build_world_objects;
pub use may_clear_density_dir::clear_density_dir_if_rebuilding;
use may_clear_density_dir::compact;
pub use may_clear_density_dir::gunzip;
pub use may_clear_density_dir::gz9;
pub use may_clear_density_dir::may_clear_density_dir;
pub use may_clear_density_dir::phase_kinds;
use may_clear_density_dir::pretty_nl;
pub use may_clear_density_dir::terrain_row;

#[path = "chunk_partitioner/build_world_objects_opt.rs"]
mod build_world_objects_opt;
pub use build_world_objects_opt::build_world_objects_opt;

#[path = "chunk_partitioner/redensify_from_committed.rs"]
mod redensify_from_committed;
pub use redensify_from_committed::build_roads_from_topo;
pub use redensify_from_committed::build_roads_from_topo_opt;
pub use redensify_from_committed::gen_density_fixture;
pub use redensify_from_committed::redensify_from_committed;
pub use redensify_from_committed::road_census;
