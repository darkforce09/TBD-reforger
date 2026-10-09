//! The terrain heightfield a lake or pond's depth is measured against.
//!
//! **Role:** [`DemHeightfield`] holds a 16-bit grey elevation PNG decoded with
//! `terrain_elevation`'s decoder ([`DemHeightfield::load`]) and answers the terrain height under a
//! world point by nearest sample ([`DemHeightfield::elevation_m`]).
//! **Position:** loaded by the vector rasterizer when `--dem` names a PNG; the polygon rasterizer
//! deepens a lake or pond sample to the water column above the terrain.
//! **Signals & state:** the heightfield owns its decoded samples; nothing else is held.
//! **Invariants:** a sample covers 2 m of world on each axis, sample `(0, 0)` at the world origin;
//! the sample under `(x, z)` is `(round(x / 2), round(z / 2))`, ties toward +∞, clamped to the
//! image (a NaN coordinate reads sample value 0); a sample value `v` is `-204.781 + (v / 65535) ×
//! (375.531 − -204.781)` metres; the PNG is decoded with its row filters undone and channel 0 read
//! big-endian; a PNG that does not decode as 16-bit, or holds no sample, is refused.

use std::path::Path;

use grid_rasterization::prelude::round_half_up;
use terrain_elevation::prelude::decode_png_gray16;

use super::water_raster::{nan_propagating_max, nan_propagating_min};
use crate::error::{Result, ResultExt, refusal};

/// The world metres one heightfield sample covers on each axis.
const DEM_METRES_PER_SAMPLE: f64 = 2.0;

/// The elevation of sample value 0, in metres.
const DEM_MINIMUM_ELEVATION_M: f64 = -204.781;

/// The elevation of sample value 65535, in metres.
const DEM_MAXIMUM_ELEVATION_M: f64 = 375.531;

/// A decoded 16-bit terrain heightfield.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DemHeightfield {
    /// Samples per row.
    width: usize,
    /// Rows.
    height: usize,
    /// The raw sample values, row-major, row 0 first as the PNG stores it.
    samples: Vec<u16>,
}

impl DemHeightfield {
    /// Decodes the 16-bit grey PNG at `path`; refuses a file that does not read or decode, and an
    /// image with no sample.
    pub(super) fn load(path: &Path) -> Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("read the DEM {}", path.display()))?;
        let (samples, width, height) = decode_png_gray16(&bytes)
            .map_err(|cause| refusal!("decode the DEM {}: {cause}", path.display()))?;
        let heightfield = Self {
            width: usize::try_from(width)?,
            height: usize::try_from(height)?,
            samples,
        };
        if heightfield.width == 0 || heightfield.height == 0 {
            return Err(refusal!("the DEM {} holds no sample", path.display()));
        }
        Ok(heightfield)
    }

    /// The sample count along each axis, `(width, height)`.
    pub(super) fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// The terrain height in metres under world point `(x, z)`: the nearest sample, clamped to
    /// the image.
    pub(super) fn elevation_m(&self, x: f64, z: f64) -> f64 {
        let sample_value = match (sample_index(x, self.width), sample_index(z, self.height)) {
            (Some(column), Some(row)) => self.samples[row * self.width + column],
            _ => 0,
        };
        DEM_MINIMUM_ELEVATION_M
            + (f64::from(sample_value) / 65535.0)
                * (DEM_MAXIMUM_ELEVATION_M - DEM_MINIMUM_ELEVATION_M)
    }
}

/// The index of the sample nearest world coordinate `coordinate` on an axis of `count` samples,
/// clamped to `0..count`; `None` for a NaN coordinate.
fn sample_index(coordinate: f64, count: usize) -> Option<usize> {
    let last = count as f64 - 1.0;
    let index = nan_propagating_min(
        last,
        nan_propagating_max(0.0, round_half_up(coordinate / DEM_METRES_PER_SAMPLE)),
    );
    // NaN fails the comparison; otherwise a whole number in `0..count`, so the cast is exact.
    (index >= 0.0).then_some(index as usize)
}

#[cfg(test)]
#[path = "../tests/water_export_images_dem_sampling_tests.rs"]
mod tests;
