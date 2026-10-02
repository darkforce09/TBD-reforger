//! **Role:** the terrain's elevation raster at its native 2 m sample spacing, kept as the
//! exported `u16` samples, with bilinear height lookup in map metres.
//! **Position:** `world/terrain/dem`. The terrain boot path (`streaming::host::bootstrap`) fills
//! a [`FullResolutionDemHandle`] when its scope keeps the raster; frontend map views and fire
//! planning read heights from it. The box-averaged [`crate::world::terrain::dem::grid::DemVectorGrid`]
//! stays the contour and line-of-sight source; this raster is the precise-height source.
//! **Signals & state:** none; an immutable raster behind an `Rc` once published into the handle.
//! **Invariants:** sample `(0, 0)` sits at the footprint's `(min_x, min_y)` corner and sample
//! `(width - 1, height - 1)` at `(max_x, max_y)`, the same row and column orientation the vector
//! grid uses; elevation is `offset_m + sample · scale_m`; a lookup outside the footprint, or at
//! a non-finite coordinate, is `None`, never a clamped edge value.

use std::cell::RefCell;
use std::rc::Rc;

/// Largest `u16` sample, the top of the linear PNG height range.
const U16_SAMPLE_MAX: f64 = 65_535.0;

/// Linear mapping from a raw `u16` sample to metres above sea level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampleEncoding {
    /// Metres at sample value zero.
    pub offset_m: f64,

    /// Metres per sample step.
    pub scale_m: f64,
}

impl SampleEncoding {
    /// The 16-bit PNG export encoding: sample `0` is `min_m`, sample `65535` is `max_m`.
    #[must_use]
    pub fn linear_range(min_m: f64, max_m: f64) -> Self {
        Self {
            offset_m: min_m,
            scale_m: (max_m - min_m) / U16_SAMPLE_MAX,
        }
    }

    /// Metres for a (possibly interpolated) sample value.
    #[must_use]
    pub fn metres(&self, sample: f64) -> f64 {
        self.offset_m + sample * self.scale_m
    }
}

/// World rectangle a raster covers, in map metres (x east, y north).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterFootprint {
    /// West edge.
    pub min_x: f64,

    /// South edge.
    pub min_y: f64,

    /// East edge.
    pub max_x: f64,

    /// North edge.
    pub max_y: f64,
}

impl RasterFootprint {
    /// Footprint from a manifest `worldBounds` array `[min_x, min_y, max_x, max_y]`.
    #[must_use]
    pub fn from_world_bounds(bounds: [f64; 4]) -> Self {
        Self {
            min_x: bounds[0],
            min_y: bounds[1],
            max_x: bounds[2],
            max_y: bounds[3],
        }
    }

    fn is_usable(&self) -> bool {
        [self.min_x, self.min_y, self.max_x, self.max_y]
            .iter()
            .all(|v| v.is_finite())
            && self.max_x > self.min_x
            && self.max_y > self.min_y
    }
}

/// The whole elevation raster at native resolution; see the module header for its frame.
#[derive(Clone, Debug, PartialEq)]
pub struct FullResolutionDem {
    samples: Vec<u16>,
    width: u32,
    height: u32,
    encoding: SampleEncoding,
    footprint: RasterFootprint,
}

impl FullResolutionDem {
    /// Wrap a row-major raster. `None` when the sample count is not `width · height`, either
    /// dimension is below two samples, the encoding is not finite, or the footprint is empty.
    #[must_use]
    pub fn new(
        samples: Vec<u16>,
        width: u32,
        height: u32,
        encoding: SampleEncoding,
        footprint: RasterFootprint,
    ) -> Option<Self> {
        let expected = (width as usize).checked_mul(height as usize)?;
        if width < 2 || height < 2 || samples.len() != expected {
            return None;
        }
        if !(encoding.offset_m.is_finite() && encoding.scale_m.is_finite()) {
            return None;
        }
        if !footprint.is_usable() {
            return None;
        }
        Some(Self {
            samples,
            width,
            height,
            encoding,
            footprint,
        })
    }

    /// Samples per row.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Rows.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The world rectangle the raster covers.
    #[must_use]
    pub fn footprint(&self) -> RasterFootprint {
        self.footprint
    }

    /// Bytes the raster keeps resident.
    #[must_use]
    pub fn resident_bytes(&self) -> u64 {
        self.samples.len() as u64 * std::mem::size_of::<u16>() as u64
    }

    /// Bilinear terrain height in metres at map position `(x, y)`; `None` outside the footprint
    /// or at a non-finite coordinate.
    #[must_use]
    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        if !(x.is_finite() && y.is_finite()) {
            return None;
        }
        let f = self.footprint;
        if x < f.min_x || x > f.max_x || y < f.min_y || y > f.max_y {
            return None;
        }
        let cols = self.width as usize;
        let rows = self.height as usize;
        let px = (x - f.min_x) / (f.max_x - f.min_x) * (cols - 1) as f64;
        let py = (y - f.min_y) / (f.max_y - f.min_y) * (rows - 1) as f64;
        let sample = crate::world::terrain::dem::sampling::bilinear_sample(
            &self.samples,
            cols,
            rows,
            px,
            py,
        );
        Some(self.encoding.metres(sample))
    }
}

/// Shared slot a terrain boot publishes its [`FullResolutionDem`] into.
pub type FullResolutionDemHandle = Rc<RefCell<Option<Rc<FullResolutionDem>>>>;

/// An empty [`FullResolutionDemHandle`].
#[must_use]
pub fn new_full_resolution_dem_handle() -> FullResolutionDemHandle {
    Rc::new(RefCell::new(None))
}

/// Height at `(x, y)` from whatever raster `handle` currently holds; `None` before the raster
/// has loaded or outside it.
#[must_use]
pub fn height_from_handle(handle: &FullResolutionDemHandle, x: f64, y: f64) -> Option<f64> {
    handle.borrow().as_ref()?.height_at(x, y)
}

#[cfg(test)]
#[path = "tests/full_resolution_tests.rs"]
mod tests;
