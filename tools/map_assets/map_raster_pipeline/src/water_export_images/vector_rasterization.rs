//! The export's lake, pond and river vectors drawn over the water raster.
//!
//! **Role:** [`rasterize_water_vectors`] reads the lake, pond and river vector files that are
//! present, loads the `--dem` heightfield when one is named, and draws the lakes, then the ponds,
//! then the rivers ([`VectorCounts`]).
//! **Position:** called by the lane's `run` after the export grids are decoded, unless
//! `--no-vector-enhance` is given or no vector file is present; draws through
//! `polygon_rasterization.rs` and `river_rasterization.rs`.
//! **Signals & state:** none held; writes the caller's [`WaterRaster`].
//! **Invariants:** the lakes are the first given of a lake document's `lakes` and `waterBodies`,
//! the ponds its `ponds`, the rivers its `rivers` (an empty array counts as given; a document
//! that is not an object gives none); lakes and ponds are water class 2 and rivers class 3, and a
//! later layer overwrites the class of an earlier one; a vector file that does not read or parse,
//! and a named heightfield that does not decode, are refused.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::dem_sampling::DemHeightfield;
use super::export_location::WaterExportFiles;
use super::json_field_reading::first_given;
use super::polygon_rasterization::fill_water_polygons;
use super::river_rasterization::ribbon_rivers;
use super::water_raster::WaterRaster;
use crate::error::{Result, ResultExt};

/// The water class byte of a lake or pond sample.
const LAKE_OR_POND_CLASS_CODE: u8 = 2;

/// How many vectors of each kind were drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct VectorCounts {
    /// Lake polygons drawn.
    pub(super) lakes: usize,
    /// Pond polygons drawn.
    pub(super) ponds: usize,
    /// River segments drawn.
    pub(super) river_segments: usize,
}

/// Draws the vectors of `files` into `raster`, with the heightfield at `dem_path` when given;
/// see the module invariants.
pub(super) fn rasterize_water_vectors(
    files: &WaterExportFiles,
    dem_path: Option<&Path>,
    raster: &mut WaterRaster,
) -> Result<VectorCounts> {
    let lakes = vector_list(files.lakes_path.as_ref(), &["lakes", "waterBodies"])?;
    let ponds = vector_list(files.ponds_path.as_ref(), &["ponds"])?;
    let rivers = vector_list(files.rivers_path.as_ref(), &["rivers"])?;
    let heightfield = dem_path.map(DemHeightfield::load).transpose()?;
    if let (Some(heightfield), Some(path)) = (&heightfield, dem_path) {
        let (width, height) = heightfield.size();
        println!(
            "Loaded DEM heightfield ({width}x{height}) from {}",
            path.display()
        );
    }
    let heightfield = heightfield.as_ref();
    Ok(VectorCounts {
        lakes: fill_water_polygons(&lakes, LAKE_OR_POND_CLASS_CODE, heightfield, raster),
        ponds: fill_water_polygons(&ponds, LAKE_OR_POND_CLASS_CODE, heightfield, raster),
        river_segments: ribbon_rivers(&rivers, raster)?,
    })
}

/// The first given of `keys` in the JSON document at `path`, or an empty array when there is no
/// file or the document gives none of them.
fn vector_list(path: Option<&PathBuf>, keys: &[&str]) -> Result<Value> {
    let Some(path) = path else {
        return Ok(Value::Array(Vec::new()));
    };
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let document: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(first_given(&document, keys)
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new())))
}
