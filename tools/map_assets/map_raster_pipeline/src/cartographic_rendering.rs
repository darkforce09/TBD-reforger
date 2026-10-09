//! The cartographic lane: the land-cover masks (forest and bright masks by orthophoto appearance),
//! the cartographic render (TGA base → land-cover tints → Lanczos upscale → inland-water tint →
//! `.topo` road strokes through resvg), and the tile pyramid builder (XYZ WebP levels; lossless
//! through `image-webp`, lossy through the vendored libwebp of the `webp` crate).
//!
//! **Role:** the land-cover thresholds and tint colours; declares the mask, render, pyramid and
//! manifest modules.
//! **Position:** a lane of the crate; the command line calls the subcommands it re-exports.
//! **Signals & state:** none; constants and plain data.
//! **Invariants:** the thresholds and tints are named constants here.

use std::path::{Path, PathBuf};

use crate::error::{Result, bail};
use serde_json::{Value, json};

use super::image_operations::{self, Rgb8};
use ::repository_root::find_repository_root;
use enfusion_pak::PakVfs;
use time_source::iso_from_system_time;
use world_export_pipeline::json_number_formatting::js_num;
use world_export_pipeline::topo::decode_topo;

pub(crate) const CLASS_RASTER_PIXELS: usize = 3200;
const FOREST_LUM_MAX: f64 = 52.0;
const FOREST_GREEN_OVER_BLUE: u16 = 8;
const BRIGHT_RED_OVER_GREEN: u16 = 4;
const BRIGHT_LUM_MIN: f64 = 58.0;

pub(crate) struct LandcoverOutputs {
    pub(crate) forest_mask: PathBuf,
    pub(crate) bright_mask: PathBuf,
    pub(crate) meta: Value,
}

/* ─────────────────────────── build-map-cartographic ─────────────────────────── */

const WATER_COLOR: [f64; 3] = [0x2e as f64, 0x52 as f64, 0x66 as f64];
const OPEN_TINT: ([f64; 3], f64) = ([0xcd as f64, 0xc6 as f64, 0xa3 as f64], 0.7);
const FOREST_TINT: ([f64; 3], f64) = ([0x37 as f64, 0x50 as f64, 0x2d as f64], 0.8);

/* ─────────────────────────── build-tile-pyramid ─────────────────────────── */

/* ─────────────────────────── manifest inline-patch helpers ─────────────────────────── */

/* ─────────────────────────── verify-cartographic ─────────────────────────── */

mod build_landcover_masks;
pub(crate) use build_landcover_masks::build_landcover_command_line;
pub(crate) use build_landcover_masks::build_map_cartographic;

mod build_tile_pyramid;
pub(crate) use build_tile_pyramid::build_tile_pyramid;
pub(crate) use build_tile_pyramid::patch_map_tiles_meta;
pub(crate) use build_tile_pyramid::patch_unified_bytes;
pub(crate) use build_tile_pyramid::reset_water_meta;
pub(crate) use build_tile_pyramid::verify_cartographic;
