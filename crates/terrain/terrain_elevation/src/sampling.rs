//! Sample conversion and bilinear sampling of the elevation raster.
//!
//! **Role:** converts a stored `u16` to metres ([`uint16_to_meters`], [`meters_cache`]), maps a
//! world point onto continuous pixel coordinates ([`world_to_pixel`]) and interpolates a raster
//! bilinearly ([`bilinear_sample`], [`sample_elevation_meters`],
//! [`sample_elevation_from_meters_cache`]).
//! **Position:** the base of the crate: [`crate::png`], [`crate::grid`] and
//! [`crate::full_resolution`] convert and interpolate through it; the line of sight, the spot
//! heights and the developer tools sample with it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a stored 0 reads exactly the minimum height and 65 535 the maximum; the
//! rectangle's far corner lands on the last pixel; a point off the raster samples as `None`.

use crate::manifest::DemManifest;
use crate::manifest::PixelCoord;

/// `uint16`-linear sample → meters ASL (Bohemia Terrain Creation Tool encoding). Mirror of `uint16ToMeters` (`sampleElevation.ts:9`): `minM + (u16/65535)*(maxM - minM)`.
#[inline]
#[must_use]
pub fn uint16_to_meters(u16v: f64, min_m: f64, max_m: f64) -> f64 {
    min_m + (u16v / 65535.0) * (max_m - min_m)
}

/// Converts every stored `u16` sample to `f32` metres through [`uint16_to_meters`], keeping
/// the raster's row-major order; the cache the runtime samplers read.
#[must_use]
pub fn meters_cache(raster: &[u16], min_m: f64, max_m: f64) -> Vec<f32> {
    raster
        .iter()
        .map(|&u| uint16_to_meters(f64::from(u), min_m, max_m) as f32)
        .collect()
}

/// World meters (x, z) → continuous pixel coords. Mirror of `worldToPixel` (`sampleElevation.ts:17`).
#[must_use]
pub fn world_to_pixel(x: f64, z: f64, m: &DemManifest) -> PixelCoord {
    let w_m = m.max_x - m.min_x;
    let h_m = m.max_y - m.min_y;
    let mut u = (x - m.min_x) / w_m;
    let mut v = (z - m.min_y) / h_m;
    if m.flip_x {
        u = 1.0 - u;
    }
    if m.flip_z {
        v = 1.0 - v;
    }
    PixelCoord {
        u,
        v,
        px: u * (m.width_px as f64 - 1.0),
        py: v * (m.height_px as f64 - 1.0),
    }
}

/// Bilinear sample of a row-major `width × height` raster. Mirror of `bilinearSample` (`sampleElevation.ts:39`) — generic over the element type (`u16` or `f32`), read as `f64`. Caller guarantees `px ∈ [0, width-1]`, `py ∈ [0, height-1]` (see `sample_elevation_meters`).
#[must_use]
pub fn bilinear_sample<T>(raster: &[T], width: usize, height: usize, px: f64, py: f64) -> f64
where
    T: Copy + Into<f64>,
{
    let x0 = px.floor();
    let y0 = py.floor();
    let x0u = x0 as usize;
    let y0u = y0 as usize;
    let x1u = (x0u + 1).min(width - 1);
    let y1u = (y0u + 1).min(height - 1);
    let fx = px - x0;
    let fy = py - y0;
    let at = |y: usize, xx: usize| -> f64 { raster[y * width + xx].into() };
    let v00 = at(y0u, x0u);
    let v10 = at(y0u, x1u);
    let v01 = at(y1u, x0u);
    let v11 = at(y1u, x1u);
    let top = v00 * (1.0 - fx) + v10 * fx;
    let bot = v01 * (1.0 - fx) + v11 * fx;
    top * (1.0 - fy) + bot * fy
}

/// Bilinear on the `uint16` grid, then convert to meters. Mirror of `sampleElevationMeters` (`sampleElevation.ts:67`). `None` on out-of-bounds (the TS throws; the runtime `DemController.sampleElevation` clamps first so it never does).
#[must_use]
pub fn sample_elevation_meters<T>(
    x: f64,
    z: f64,
    m: &DemManifest,
    raster: &[T],
    width: usize,
    height: usize,
) -> Option<f64>
where
    T: Copy + Into<f64>,
{
    let pc = world_to_pixel(x, z, m);
    if pc.px < 0.0 || pc.py < 0.0 || pc.px > width as f64 - 1.0 || pc.py > height as f64 - 1.0 {
        return None;
    }
    let u16v = bilinear_sample(raster, width, height, pc.px, pc.py);
    Some(uint16_to_meters(u16v, m.height_min_m, m.height_max_m))
}

/// Bilinear sample on the **f32 meters cache** (runtime DEM). Mirror of `bilinearSample` on the meters `Float32Array` — no second `uint16_to_meters` pass (that path is for raw u16 rasters).
#[must_use]
pub fn sample_elevation_from_meters_cache(
    x: f64,
    z: f64,
    m: &DemManifest,
    meters: &[f32],
    width: usize,
    height: usize,
) -> Option<f64> {
    let pc = world_to_pixel(x, z, m);
    if pc.px < 0.0 || pc.py < 0.0 || pc.px > width as f64 - 1.0 || pc.py > height as f64 - 1.0 {
        return None;
    }
    Some(bilinear_sample(meters, width, height, pc.px, pc.py))
}

/// Whether world point `(x, y)` lies inside the manifest's rectangle, edges included; `y` is
/// the world Z axis the manifest's `min_y`/`max_y` span.
#[inline]
#[must_use]
pub fn in_coverage(m: &DemManifest, x: f64, y: f64) -> bool {
    x >= m.min_x && x <= m.max_x && y >= m.min_y && y <= m.max_y
}

#[cfg(test)]
#[path = "tests/sampling_tests.rs"]
mod tests;
