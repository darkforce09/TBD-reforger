//! T-165.9 — the cartographic lane: `build-landcover-mask.mjs` (SAP-appearance forest/bright
//! masks), `build-map-cartographic.mjs` (TGA base → landcover tints → Lanczos upscale →
//! inland-water tint → .topo road strokes via resvg — replaces the magick MVG draw pass),
//! and the tile-pyramid builder (`build-tile-pyramid.sh` — XYZ WebP levels; lossless via
//! image-webp, the lossy leg via the vendored-libwebp `webp` crate per N3).

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::image_operations::{self, Rgb8};
use crate::browser_testing::server::repo_root;
use crate::enfusion_pak::PakVfs;
use crate::timestamp_formatting::iso_from_system_time;
use crate::world_export_pipeline::json_number_formatting::js_num;
use crate::world_export_pipeline::topo::decode_topo;

pub const CLASS_PX: usize = 3200;
const FOREST_LUM_MAX: f64 = 52.0;
const FOREST_GREEN_OVER_BLUE: u16 = 8;
const BRIGHT_RED_OVER_GREEN: u16 = 4;
const BRIGHT_LUM_MIN: f64 = 58.0;

pub struct LandcoverOut {
    pub forest_mask: PathBuf,
    pub bright_mask: PathBuf,
    pub meta: Value,
}

/* ─────────────────────────── build-map-cartographic ─────────────────────────── */

const WATER_COLOR: [f64; 3] = [0x2e as f64, 0x52 as f64, 0x66 as f64];
const OPEN_TINT: ([f64; 3], f64) = ([0xcd as f64, 0xc6 as f64, 0xa3 as f64], 0.7);
const FOREST_TINT: ([f64; 3], f64) = ([0x37 as f64, 0x50 as f64, 0x2d as f64], 0.8);

/* ─────────────────────────── build-tile-pyramid ─────────────────────────── */

/* ─────────────────────────── Makefile inline-patch helpers (were `node -e`) ─────────────────────────── */

/* ─────────────────────────── verify-t152-cartographic ─────────────────────────── */

#[path = "cartographic_rendering/build_landcover_masks.rs"]
mod build_landcover_masks;
pub use build_landcover_masks::build_landcover_cli;
pub use build_landcover_masks::build_landcover_masks;
pub use build_landcover_masks::build_map_cartographic;

#[path = "cartographic_rendering/build_tile_pyramid.rs"]
mod build_tile_pyramid;
pub use build_tile_pyramid::build_tile_pyramid;
pub use build_tile_pyramid::patch_map_tiles_meta;
pub use build_tile_pyramid::patch_unified_bytes;
pub use build_tile_pyramid::reset_water_meta;
pub use build_tile_pyramid::verify_t152;
