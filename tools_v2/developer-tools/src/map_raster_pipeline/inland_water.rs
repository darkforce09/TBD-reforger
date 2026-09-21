//! The water lane: the source analysis (the inland-water
//! classifier — grey/wet pixel classes on the road-subtracted field, component acceptance,
//! mask + preview + spike JSON) and the composite (ocean ramp + inland tint
//! over the SAP ortho, inward feather, in-place with backup + meta block).

mod source_components;

use source_components::WaterSourceComponent;
use std::path::PathBuf;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::image_operations::{self, Rgb8};
use crate::browser_testing::server::repo_root;
use crate::enfusion_pak::PakVfs;
use crate::timestamp_formatting::iso_from_system_time;
use crate::world_export_pipeline::json_number_formatting::{js_math_round, js_num};
use crate::world_export_pipeline::topo::{TOPO_AIRFIELD, decode_topo};

/* ─────────────────────────── composite-water-ortho ─────────────────────────── */

const OCEAN_BRIGHT: [f64; 3] = [58.0, 96.0, 120.0];
const OCEAN_DARK: [f64; 3] = [28.0, 52.0, 78.0];
const INLAND_COLOR: [f64; 3] = [52.0, 88.0, 112.0];
const WATER_ALPHA: f64 = 0.8;
const DEPTH_FULL_M: f64 = 80.0;
const FEATHER_R: usize = 3;

/* ─────────────────────────── analyze-water-sources ─────────────────────────── */

const DETECT_DIM: usize = 3200;
const SAT_MAX: f32 = 0.12;
const LUM_MIN: f32 = 0.2;
const LUM_MAX: f32 = 0.44;
const OPEN_R: usize = 2;
const DENSITY_MIN: f64 = 0.6;
const OCEAN_DILATE_R: usize = 5;
const FLAT_DILATE_R: usize = 2;
const MIN_AREA_M2: u64 = 2000;
const MEAN_SAT_MAX: f64 = 0.115;
const SLOPE_PX_MAX_DEG: f32 = 18.0;
const FLAT_FRAC_MAX: f64 = 0.12;
const SLOPE_MEAN_MAX_DEG: f64 = 8.0;
const ROAD_SAMPLE_STEP_PX: f64 = 2.0;
const ROAD_OVERLAP_MAX: f64 = 0.45;
const RIBBON_W_MAX_PX: f64 = 5.0;
const LIN_MIN_AREA_M2: u64 = 800;
const LIN_SLOPE_MEAN_MAX_DEG: f64 = 16.0;
const LIN_FLAT_FRAC_MAX: f64 = 0.2;
const GREY_RIVER_VALLEY_MIN: f64 = 0.2;
const GREY_RIVER_LOWLAND_SLOPE_DEG: f64 = 8.0;
const WET_MIN_AREA_M2: u64 = 1000;
const WET_VALLEY_FRAC_MIN: f64 = 0.6;
const WET_MEAN_SAT_MAX: f64 = 0.18;
const WET_MEAN_LUM_MAX: f64 = 0.31;
const WET_LUM_MIN: f32 = 0.09;
const WET_LUM_MAX: f32 = 0.33;
const WET_SAT_MAX: f32 = 0.19;
const WET_SLOPE_PX_MAX_DEG: f32 = 24.0;
const VALLEY_BLUR_R: usize = 12;
const VALLEY_CARVE_M: f32 = 0.8;

// valleyCarveM prints as 0.8 in JS.
const WET_VALLEY_CARVE_JSON: f64 = 0.8;

#[path = "inland_water/sap_dir.rs"]
mod sap_dir;
pub use sap_dir::composite_water_ortho;
use sap_dir::dilate;
use sap_dir::read_dem_u16;
use sap_dir::sap_dir;

#[path = "inland_water/analyze_water_sources.rs"]
mod analyze_water_sources;
pub use analyze_water_sources::analyze_water_sources;
