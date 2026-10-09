//! The water class mask and the depth grid every water image is drawn from.
//!
//! **Role:** [`WaterRaster`] holds one water class byte and one depth in decimetres per sample
//! of the image grid, row 0 the southern row (the export's order), and merges a rasterized depth
//! keeping the deeper ([`WaterRaster::deepen`]); [`nan_propagating_min`] and
//! [`nan_propagating_max`] are the minimum and maximum the depth rules take, where a NaN operand
//! gives NaN.
//! **Position:** filled by the grid decoder and the polygon and river rasterizers, read by the
//! statistics and the image writers (`water_image_outputs.rs`).
//! **Signals & state:** the raster owns its two sample vectors; nothing else is held.
//! **Invariants:** both vectors hold `width × height` samples, sample `(column, row)` at
//! `row × width + column`; a depth in metres becomes `min(65535, round(depth × 10))` decimetres,
//! ties toward +∞, and is stored only when it is greater than the stored depth, so a NaN or a
//! negative depth never stores and a stored depth is always a whole number in `0..=65535`.

use grid_rasterization::prelude::{SampleGrid, round_half_up};

/// The largest depth in decimetres a sample holds.
const MAXIMUM_DEPTH_DECIMETRES: f64 = 65535.0;

/// The water class mask and the depth grid of the image grid.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WaterRaster {
    /// The world placement and size of the samples.
    pub(super) grid: SampleGrid,
    /// One water class byte per sample (0 land, 1 sea, 2 lake or pond, 3 river).
    pub(super) mask: Vec<u8>,
    /// One depth in decimetres per sample.
    pub(super) depth_decimetres: Vec<u16>,
}

impl WaterRaster {
    /// An all-land raster of zero depth over `grid`.
    pub(super) fn new(grid: SampleGrid) -> Self {
        let samples = grid.width * grid.height;
        Self {
            grid,
            mask: vec![0; samples],
            depth_decimetres: vec![0; samples],
        }
    }

    /// The sample index of `(column, row)`.
    pub(super) fn index(&self, column: usize, row: usize) -> usize {
        row * self.grid.width + column
    }

    /// Stores `depth_m` at sample `index` as `min(65535, round(depth_m × 10))` decimetres when
    /// that is greater than the depth stored there.
    pub(super) fn deepen(&mut self, index: usize, depth_m: f64) {
        let depth_decimetres =
            nan_propagating_min(MAXIMUM_DEPTH_DECIMETRES, round_half_up(depth_m * 10.0));
        if depth_decimetres > f64::from(self.depth_decimetres[index]) {
            // Greater than a stored u16 and at most 65535, and whole: the cast is exact.
            self.depth_decimetres[index] = depth_decimetres as u16;
        }
    }
}

/// The smaller of `first` and `second`, or NaN when either is NaN ([`f64::min`] drops a NaN
/// operand instead).
pub(super) fn nan_propagating_min(first: f64, second: f64) -> f64 {
    if first.is_nan() || second.is_nan() {
        f64::NAN
    } else {
        first.min(second)
    }
}

/// The larger of `first` and `second`, or NaN when either is NaN ([`f64::max`] drops a NaN
/// operand instead).
pub(super) fn nan_propagating_max(first: f64, second: f64) -> f64 {
    if first.is_nan() || second.is_nan() {
        f64::NAN
    } else {
        first.max(second)
    }
}
