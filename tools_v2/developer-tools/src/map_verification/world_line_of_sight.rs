//! `cargo xtask map world-los` — T-090.12.3: the world occluder on the committed catalogue,
//! engine-free. Loads `objects/prefabs.json.gz`, a cell and its 8 neighbours from
//! `objects/chunks/`, and every descriptor + BLAS the rows need from `prefabs/`, then:
//!
//! - `--census`            per-chunk kinds, proxy rows, pending BLAS, memory
//! - `--probe ax,ay,az bx,by,bz`   one segment (ENGINE frame `[x, y_up, z_north]`): events + verdict
//! - `--bench N`           N random eye-height segments in the cell: µs / segment
//! - `--pairs <json>`      replay a `world-parity` oracle (T-090.12.4): agree / phantom / missed
//!   per policy, bucketed by the engine's hit prefab kind; `--min-agree F` exits 1 below it
//! - `--dem`               also replay the `clearWorld` column: objects ∧ the 2 m DEM (terrain
//!   sampled every metre along the pair through the editor's `DemManifest` sampler)
//!
//! Usage: `--cell <cx_cy> [--assets assets_v2/terrains/everon] [--census] [--probe a b]
//!         [--bench N] [--pairs <json>] [--glass-blocks] [--foliage-blocks] [--proxy-only]
//!         [--min-agree F] [--dump-misses <jsonl>] [--dem]`

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::repository_layout::terrain_dir;
use website_map_engine::spatial::bvh::sidecar::BvhSidecar;
use website_map_engine::spatial::los::world::coverage_1::BlockPolicy;
use website_map_engine::spatial::los::world::coverage_1::WorldVerdict;
use website_map_engine::spatial::los::world::descriptor::PrefabDescriptor;
use website_map_engine::spatial::los::world::state::WorldOccluder;
use website_map_engine::streaming::loaders::chunk::parse_chunk;
use website_map_engine::streaming::loaders::chunk_bin::parse_chunk_bin_for;
use website_map_engine::streaming::scheduler::chunk_math::TerrainSizeM;
use website_map_engine::world::environment::buildings::prefab::build_prefab_maps;
use website_map_engine::world::environment::buildings::prefab::narrow_prefab_rows;
use website_map_engine::world::terrain::dem::manifest::DemManifest;
use website_map_engine::world::terrain::dem::sampling::sample_elevation_meters;

/// Everon: 12 800 m square, 512 m chunks.
pub const TERRAIN_M: f64 = 12_800.0;
pub const CHUNK_M: f64 = 512.0;

/// One oracle pair: `[ox, oy, oz, tx, ty, tz, clearEnts, clearWorld, hitPrefabSlug]` (engine frame).
pub type WorldPair = (f64, f64, f64, f64, f64, f64, bool, bool, String);

/// The terrain half of the `clearWorld` column: the committed 16-bit DEM behind the editor's own
/// `DemManifest` sampler (`dem::sample`, Class R), so the CLI and the LOS tool read the same
/// heights. 2 m pixels — fine terrain detail the engine's `WORLD` trace sees is below this
/// resolution, which is the documented caveat on the world-inclusive number.
pub struct Dem {
    pub m: DemManifest,
    pub raster: Vec<u16>,
    pub w: usize,
    pub h: usize,
}

impl Dem {
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
    pub cell: [i64; 2],
    #[serde(default)]
    pub seed: i64,
    pub pairs: Vec<WorldPair>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReplayReport {
    pub n: usize,
    pub agree: usize,
    /// Model blocked, engine clear.
    pub phantom: usize,
    /// Model clear, engine blocked.
    pub missed: usize,
    pub provisional: usize,
    /// Disagreements bucketed by the engine's hit prefab slug (empty = engine clear).
    pub by_hit: BTreeMap<String, (usize, usize)>,
    /// `--dem`: the world-inclusive column (`clearWorld` = objects ∧ terrain). `world_n == 0`
    /// when no DEM was given.
    pub world_n: usize,
    pub world_agree: usize,
    /// Model world-blocked, engine world-clear — split by which half blocked in the model.
    pub world_phantom_terrain: usize,
    pub world_phantom_objects: usize,
    /// Model world-clear, engine world-blocked.
    pub world_missed: usize,
}

impl ReplayReport {
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
#[path = "tests/world_line_of_sight.rs"]
mod tests;

#[path = "world_line_of_sight/gunzip_json.rs"]
mod gunzip_json;
pub use gunzip_json::load_cell;
pub use gunzip_json::load_dem;
pub use gunzip_json::replay;
pub use gunzip_json::run;
