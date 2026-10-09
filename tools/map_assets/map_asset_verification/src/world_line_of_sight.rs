//! `cargo xtask map world-los`: the world occluder over the committed catalogue, without the
//! map engine's streaming.
//!
//! **Role:** loads `objects/prefabs.json.gz`, a cell and its eight neighbours from
//! `objects/chunks/`, and every descriptor and BLAS the rows need from `prefabs/`, then runs the
//! modes the command line asks for:
//!
//! - `--census`: per-chunk kinds, proxy rows, pending BLAS, memory;
//! - `--probe ax,ay,az bx,by,bz`: one segment (engine frame `[x, y_up, z_north]`), its events and
//!   verdict;
//! - `--bench N`: N random eye-height segments in the cell, microseconds per segment;
//! - `--pairs <json>`: replays a `world-parity` oracle (agree, phantom, missed per policy,
//!   bucketed by the engine's hit prefab kind); `--min-agree F` exits 1 below it;
//! - `--dem`: also replays the `clearWorld` column, objects and the 2 m elevation model (terrain
//!   sampled every metre along the pair through the editor's `DemManifest` sampler).
//!
//! Usage: `--cell <cx_cy> [--assets assets/terrains/everon] [--census] [--probe a b]
//! [--bench N] [--pairs <json>] [--glass-blocks] [--foliage-blocks] [--proxy-only]
//! [--min-agree F] [--dump-misses <jsonl>] [--dem]`
//!
//! **Position:** called by the `cargo xtask map world-los` adapter with the checkout root; reads
//! a terrain folder under `assets/terrains/` and an oracle file; its pinned replays in
//! `tests/world_line_of_sight.rs` read the blueprint compiler's world-parity fixtures.
//! **Signals & state:** none; each run builds its occluder afresh.
//! **Invariants:** the occluder is the `world_line_of_sight` crate's, unmodified, so a replay
//! measures the map's own line of sight; `--dump-misses` is the only file the command writes.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Result, ResultExt, bail, refusal};
use serde_json::Value;

use ::repository_layout::terrain_dir;
use map_coordinates::chunk_math::TerrainSizeM;
use prefab_catalog::prefab_rows::build_prefab_maps;
use prefab_catalog::prefab_rows::narrow_prefab_rows;
use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;
use terrain_elevation::manifest::DemManifest;
use terrain_elevation::sampling::sample_elevation_meters;
use world_chunks::chunk_container::parse_chunk_bin_for;
use world_chunks::world_chunk::parse_chunk;
use world_line_of_sight::BlockPolicy;
use world_line_of_sight::WorldOccluder;
use world_line_of_sight::WorldVerdict;
use world_line_of_sight::occluder_library::PrefabDescriptor;

/// The side of Everon's square terrain, in metres.
pub const TERRAIN_SIZE_METERS: f64 = 12_800.0;
/// The side of one streamed chunk, in metres.
pub const CHUNK_SIZE_METERS: f64 = 512.0;

/// One oracle pair: `[ox, oy, oz, tx, ty, tz, clearEnts, clearWorld, hitPrefabSlug]` (engine frame).
pub type WorldOraclePair = (f64, f64, f64, f64, f64, f64, bool, bool, String);

/// The terrain half of the `clearWorld` column: the committed 16-bit DEM behind the editor's own
/// `DemManifest` sampler (`dem::sampling`, Class R), so the CLI and the LOS tool read the same
/// heights. 2 m pixels — fine terrain detail the engine's `WORLD` trace sees is below this
/// resolution, which is the documented caveat on the world-inclusive number.
pub struct ElevationRaster {
    /// The world bounds, raster size, axis flips and height range of the raster.
    pub m: DemManifest,
    /// The 16-bit height samples, row-major.
    pub raster: Vec<u16>,
    /// The raster width, in pixels.
    pub w: usize,
    /// The raster height, in pixels.
    pub h: usize,
}

impl ElevationRaster {
    /// Ground height (m ASL) at engine `(x, z_north)`, `None` off the raster.
    #[must_use]
    pub fn ground(&self, x: f64, z: f64) -> Option<f64> {
        sample_elevation_meters(x, z, &self.m, &self.raster, self.w, self.h)
    }

    /// Does the terrain cut the segment? Interior samples every metre (the endpoints stand on
    /// their own ground and are skipped); blocked when the surface rises above the line.
    #[must_use]
    pub fn blocks(&self, obs: [f64; 3], tgt: [f64; 3]) -> bool {
        let d = [tgt[0] - obs[0], tgt[1] - obs[1], tgt[2] - obs[2]];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = (len.ceil() as usize).max(2);
        (1..n).any(|i| {
            let t = i as f64 / n as f64;
            let p = [obs[0] + d[0] * t, obs[1] + d[1] * t, obs[2] + d[2] * t];
            self.ground(p[0], p[2]).is_some_and(|g| g > p[1])
        })
    }
}

/// One oracle file of the `world-parity` action.
#[derive(serde::Deserialize)]
pub struct WorldParityFile {
    /// The chunk cell the pairs were drawn in.
    pub cell: [i64; 2],
    /// The seed the oracle drew the pairs with.
    #[serde(default)]
    pub seed: i64,
    /// The recorded segments and the engine's verdicts on them.
    pub pairs: Vec<WorldOraclePair>,
}

/// The tally of one oracle replay: agreement on the objects-only column, and on the
/// world-inclusive column when an elevation model was given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReplayReport {
    /// The pairs replayed.
    pub n: usize,
    /// The pairs on which the model and the engine agree.
    pub agree: usize,
    /// Model blocked, engine clear.
    pub phantom: usize,
    /// Model clear, engine blocked.
    pub missed: usize,
    /// Pairs whose verdict is provisional (a descriptor or BLAS was not resident).
    pub provisional: usize,
    /// Disagreements bucketed by the engine's hit prefab slug (empty = engine clear).
    pub by_hit: BTreeMap<String, (usize, usize)>,
    /// `--dem`: the world-inclusive column (`clearWorld` = objects ∧ terrain). `world_n == 0`
    /// when no DEM was given.
    pub world_n: usize,
    /// The pairs on which the world-inclusive verdicts agree.
    pub world_agree: usize,
    /// Model world-blocked by the terrain, engine world-clear.
    pub world_phantom_terrain: usize,
    /// Model world-blocked by an object only, engine world-clear.
    pub world_phantom_objects: usize,
    /// Model world-clear, engine world-blocked.
    pub world_missed: usize,
}

impl ReplayReport {
    /// The objects-only agreement, a fraction in `[0, 1]` (`0` with no pairs).
    #[must_use]
    pub fn agreement(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.agree as f64 / self.n as f64
        }
    }

    /// World-inclusive agreement (`0` without `--dem`).
    #[must_use]
    pub fn world_agreement(&self) -> f64 {
        if self.world_n == 0 {
            0.0
        } else {
            self.world_agree as f64 / self.world_n as f64
        }
    }
}

#[cfg(test)]
#[path = "tests/world_line_of_sight_tests.rs"]
mod tests;

mod occluder_loading_and_replay;
pub use occluder_loading_and_replay::load_cell_occluder;
pub use occluder_loading_and_replay::load_elevation_raster;
pub use occluder_loading_and_replay::replay;
pub use occluder_loading_and_replay::run;
