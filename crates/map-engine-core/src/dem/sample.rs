//! DEM sampling math — **Class R** (bit-identical to the TS reference
//! `apps/website/frontend/src/features/tactical-map/dem/sampleElevation.ts` and the parity oracle
//! `packages/tbd-schema/scripts/lib/dem-sample.mjs`).
//!
//! Arithmetic is `f64` in the same operation order as the JS, cast `as f32` at the same store
//! boundary as the JS `Float32Array` write, so buffer outputs compare `memcmp`-equal. `bilinear_sample`
//! is generic over the raster element type: the JS reads every sample as an f64 regardless of the
//! backing `Float64Array`(uint16 anchor path) or `Float32Array`(runtime meters), and `u16 → f64`
//! and `f32 → f64` are both exact, so a generic `Into<f64>` accessor reproduces the JS exactly.

/// DEM raster geometry + encoding — the fields of a `TerrainManifest.dem` the sampler needs.
/// Kept as plain scalars so `map-engine-core` stays serde-free; the wasm shim / backend map their
/// manifest onto this.
#[derive(Clone, Copy, Debug)]
pub struct DemManifest {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub width_px: usize,
    pub height_px: usize,
    pub flip_x: bool,
    pub flip_z: bool,
    pub height_min_m: f64,
    pub height_max_m: f64,
}

/// Continuous pixel coordinate on the heightmap (mirror of the `worldToPixel` return).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelCoord {
    pub u: f64,
    pub v: f64,
    pub px: f64,
    pub py: f64,
}

/// `uint16`-linear sample → meters ASL (Bohemia Terrain Creation Tool encoding).
/// Mirror of `uint16ToMeters` (`sampleElevation.ts:9`): `minM + (u16/65535)*(maxM - minM)`.
#[inline]
#[must_use]
pub fn uint16_to_meters(u16v: f64, min_m: f64, max_m: f64) -> f64 {
    min_m + (u16v / 65535.0) * (max_m - min_m)
}

/// Vectorized meters cache: a row-major `uint16` raster → `f32` meters. Mirror of
/// `buildMetersCache` (`DemTexture.ts:74`) — `out[i] = uint16ToMeters(raster[i], min, max)` stored
/// into a `Float32Array`. The Phase 0 boundary-proof kernel and the Phase 1 `dem::png` meters-cache
/// core (the Everon raster is 6400² = 40,960,000 samples → 163,840,000 bytes).
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

/// Bilinear sample of a row-major `width × height` raster. Mirror of `bilinearSample`
/// (`sampleElevation.ts:39`) — generic over the element type (`u16` or `f32`), read as `f64`.
/// Caller guarantees `px ∈ [0, width-1]`, `py ∈ [0, height-1]` (see `sample_elevation_meters`).
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

/// Bilinear on the `uint16` grid, then convert to meters. Mirror of `sampleElevationMeters`
/// (`sampleElevation.ts:67`). `None` on out-of-bounds (the TS throws; the runtime
/// `DemController.sampleElevation` clamps first so it never does).
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

/// Bilinear sample on the **f32 meters cache** (runtime DEM). Mirror of `bilinearSample` on the
/// meters `Float32Array` — no second `uint16_to_meters` pass (that path is for raw u16 rasters).
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

#[cfg(test)]
mod tests {
    use super::*;

    // Everon height range — packages/map-assets/everon/manifest.json.
    const MIN_M: f64 = -204.78;
    const MAX_M: f64 = 375.53;

    fn everon(width_px: usize, height_px: usize) -> DemManifest {
        DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 12800.0,
            max_y: 12800.0,
            width_px,
            height_px,
            flip_x: false,
            flip_z: false,
            height_min_m: MIN_M,
            height_max_m: MAX_M,
        }
    }

    #[test]
    fn zero_is_exact_min() {
        assert_eq!(uint16_to_meters(0.0, MIN_M, MAX_M), MIN_M);
    }

    #[test]
    fn full_scale_is_max_within_epsilon() {
        assert!((uint16_to_meters(65535.0, MIN_M, MAX_M) - MAX_M).abs() < 1e-10);
    }

    #[test]
    fn meters_cache_matches_scalar_and_stores_f32() {
        let raster: [u16; 5] = [0, 65535, 12345, 54321, 1];
        let out = meters_cache(&raster, MIN_M, MAX_M);
        assert_eq!(out.len(), raster.len());
        for (i, &u) in raster.iter().enumerate() {
            assert_eq!(out[i], uint16_to_meters(f64::from(u), MIN_M, MAX_M) as f32);
        }
    }

    #[test]
    fn world_to_pixel_endpoints() {
        let m = everon(6400, 6400);
        let a = world_to_pixel(0.0, 0.0, &m);
        assert_eq!((a.px, a.py), (0.0, 0.0));
        let b = world_to_pixel(12800.0, 12800.0, &m);
        assert_eq!((b.px, b.py), (6399.0, 6399.0));
    }

    #[test]
    fn world_to_pixel_axis_flip() {
        let mut m = everon(6400, 6400);
        m.flip_x = true;
        m.flip_z = true;
        let a = world_to_pixel(0.0, 0.0, &m);
        assert_eq!((a.px, a.py), (6399.0, 6399.0));
    }

    #[test]
    fn bilinear_2x2_center_is_mean() {
        // Synthetic 2×2 (mirrors sampleElevation.test.ts): corners 0,100,200,300; center = 150.
        let raster: [f32; 4] = [0.0, 100.0, 200.0, 300.0];
        let v = bilinear_sample(&raster, 2, 2, 0.5, 0.5);
        assert!((v - 150.0).abs() < 1e-9);
    }

    #[test]
    fn bilinear_u16_and_f32_agree_when_exact() {
        let u: [u16; 4] = [0, 100, 200, 300];
        let f: [f32; 4] = [0.0, 100.0, 200.0, 300.0];
        for (px, py) in [(0.0, 0.0), (0.25, 0.75), (0.9, 0.1)] {
            assert_eq!(
                bilinear_sample(&u, 2, 2, px, py),
                bilinear_sample(&f, 2, 2, px, py)
            );
        }
    }

    #[test]
    fn sample_elevation_out_of_bounds_is_none() {
        let m = everon(6400, 6400);
        let raster = vec![0u16; 64]; // tiny stand-in; the OOB check fires before any read
        assert!(sample_elevation_meters(-1.0, 0.0, &m, &raster, 6400, 6400).is_none());
    }
}
