//! The size and world placement of the water images.
//!
//! **Role:** [`water_raster_geometry`] turns the export's metadata, the requested resolution and
//! the region of interest into the sample grid every image is drawn on ([`WaterRasterGeometry`]):
//! its pixel size, the world spacing of its samples, whether it is the export's own grid, and the
//! file name suffix of a region.
//! **Position:** called by the lane's `run` after the metadata is read; the grid decoder runs
//! only on the export's own grid, and the vector rasterizers and image writers draw on
//! [`WaterRasterGeometry::grid`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** the meta's `widthPx`, `heightPx` and `worldSizeM` default to 12800 and
//! `planarResolutionM` to `worldSizeM / widthPx` when not given (absent, `null` or `0`); a region
//! is `max(16, round(extent / resolution))` pixels on each axis, otherwise a requested resolution
//! makes a square of `round(worldSizeM / resolution)` pixels; samples span the world extent end to
//! end, `extent / (pixels − 1)` apart (`extent / 1` for a single pixel); every rounding sends ties
//! toward +∞; a size that is not a whole number of at least one pixel is refused.

use grid_rasterization::prelude::{SampleGrid, round_half_up};
use serde_json::Value;

use super::json_field_reading::given_number;
use super::water_raster::nan_propagating_max;
use crate::error::{Result, bail};

/// The grid size and world size a metadata file that gives none of them describes.
const DEFAULT_EXPORT_EXTENT: f64 = 12800.0;

/// The smallest side, in pixels, of a region of interest's images.
const MINIMUM_REGION_SIDE_PX: f64 = 16.0;

/// The sample grid of the water images.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WaterRasterGeometry {
    /// The world placement of the samples; its `width` and `height` are the image size.
    pub(super) grid: SampleGrid,
    /// The world metres one pixel stands for, as the size was derived from.
    pub(super) metres_per_pixel: f64,
    /// Whether the grid is the export's own (no region, the meta's pixel size), so the export's
    /// ASCII grids lay over it sample for sample.
    pub(super) is_export_grid: bool,
    /// The file name suffix of a region of interest, `-roi-<min x>_<min z>`.
    pub(super) region_suffix: Option<String>,
}

/// The sample grid of the images of the export described by `meta`, at `resolution_m_per_px`
/// when given (a positive number) and over `region_of_interest` (`[min_x, min_z, max_x, max_z]`)
/// when given; see the module invariants.
pub(super) fn water_raster_geometry(
    meta: &Value,
    resolution_m_per_px: Option<f64>,
    region_of_interest: Option<[f64; 4]>,
) -> Result<WaterRasterGeometry> {
    let meta_width = given_number(meta.get("widthPx")).unwrap_or(DEFAULT_EXPORT_EXTENT);
    let meta_height = given_number(meta.get("heightPx")).unwrap_or(DEFAULT_EXPORT_EXTENT);
    let world_size_m = given_number(meta.get("worldSizeM")).unwrap_or(DEFAULT_EXPORT_EXTENT);
    let metres_per_pixel = resolution_m_per_px.unwrap_or_else(|| {
        given_number(meta.get("planarResolutionM")).unwrap_or(world_size_m / meta_width)
    });

    let (width, height, extent, region_suffix) = match region_of_interest {
        Some([min_x, min_z, max_x, max_z]) => {
            let side = |span: f64| {
                nan_propagating_max(
                    MINIMUM_REGION_SIDE_PX,
                    round_half_up(span / metres_per_pixel),
                )
            };
            let suffix = format!(
                "-roi-{}_{}",
                whole_number_text(round_half_up(min_x)),
                whole_number_text(round_half_up(min_z))
            );
            (
                side(max_x - min_x),
                side(max_z - min_z),
                [min_x, min_z, max_x, max_z],
                Some(suffix),
            )
        }
        None => {
            let full = [0.0, 0.0, world_size_m, world_size_m];
            match resolution_m_per_px {
                Some(_) => {
                    let side = round_half_up(world_size_m / metres_per_pixel);
                    (side, side, full, None)
                }
                None => (meta_width, meta_height, full, None),
            }
        }
    };
    let is_export_grid =
        region_of_interest.is_none() && width == meta_width && height == meta_height;
    let [min_x, min_z, max_x, max_z] = extent;
    let grid = SampleGrid {
        origin_x: min_x,
        origin_z: min_z,
        spacing_x: (max_x - min_x) / sample_intervals(width),
        spacing_z: (max_z - min_z) / sample_intervals(height),
        width: pixel_count(width, "width")?,
        height: pixel_count(height, "height")?,
    };
    Ok(WaterRasterGeometry {
        grid,
        metres_per_pixel,
        is_export_grid,
        region_suffix,
    })
}

/// The number of sample intervals across `pixels` samples: `pixels − 1`, or 1 for a single
/// sample (or fewer).
fn sample_intervals(pixels: f64) -> f64 {
    if pixels > 1.0 { pixels - 1.0 } else { 1.0 }
}

/// `pixels` as a count; refuses NaN, a fraction, less than one pixel and more than an image
/// holds (`u32::MAX`).
fn pixel_count(pixels: f64, axis: &str) -> Result<usize> {
    if !(pixels >= 1.0 && pixels <= f64::from(u32::MAX) && pixels.fract() == 0.0) {
        bail!(
            "the water image {axis} would be {pixels} pixels; it must be a whole number of at least one pixel"
        );
    }
    // A whole number in 1..=u32::MAX, so the cast is exact.
    Ok(pixels as usize)
}

/// The decimal text of the whole number `value`, with `-0` written as `0` (adding `+0` turns
/// `-0` into `+0` and leaves every other value unchanged).
fn whole_number_text(value: f64) -> String {
    format!("{}", value + 0.0)
}

#[cfg(test)]
#[path = "../tests/water_export_images_geometry_tests.rs"]
mod tests;
