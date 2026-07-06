//! DEM hillshade — **Class T** (transcendental: `atan/atan2/sin/cos`). Bit-identity is impossible
//! across libm implementations, so the differential gate is "output u8 image within ≤ 1 gray level"
//! (the ULP noise is far below one level except at exact `x.5` round boundaries). The light-direction
//! constants ARE bit-identical (`PI` matches `Math.PI`). Mirror of `buildHillshadeImage`
//! (`useDemLayer.ts:28`): stride-downsample to a ≤1024 px edge, then a Horn 3×3 slope/aspect shade,
//! row-flipped to north-up RGBA.

use crate::js;
use core::f64::consts::PI;

const MAX_EDGE: usize = 1024;
const ALTITUDE_RAD: f64 = (45.0 * PI) / 180.0;
const AZIMUTH_RAD: f64 = (315.0 * PI) / 180.0;
const ZENITH_RAD: f64 = PI / 2.0 - ALTITUDE_RAD;

/// Row-flipped RGBA image + its dims. The JS wraps `data` in an `ImageData(w, h)`.
#[derive(Clone, Debug)]
pub struct Hillshade {
    pub data: Vec<u8>,
    pub w: usize,
    pub h: usize,
}

/// Build the Horn hillshade RGBA image. Mirror of `buildHillshadeImage` (`useDemLayer.ts:28`).
#[must_use]
pub fn build_hillshade_image(meters: &[f32], src_w: usize, src_h: usize) -> Hillshade {
    let scale = 1.max((src_w.max(src_h) as f64 / MAX_EDGE as f64).ceil() as usize);
    let w = 1.max(src_w / scale);
    let h = 1.max(src_h / scale);
    let cell_meters = src_w as f64 / w as f64;

    // Stride downsample into a small meters grid.
    let mut ds = vec![0f32; w * h];
    for y in 0..h {
        let sy = (src_h - 1).min(y * scale);
        for x in 0..w {
            let sx = (src_w - 1).min(x * scale);
            ds[y * w + x] = meters[sy * src_w + sx];
        }
    }

    let mut data = vec![0u8; w * h * 4];
    let at = |x: i64, y: i64| -> f64 {
        let yy = (h as i64 - 1).min(y.max(0)) as usize;
        let xx = (w as i64 - 1).min(x.max(0)) as usize;
        f64::from(ds[yy * w + xx])
    };

    for y in 0..h {
        for x in 0..w {
            let (xi, yi) = (x as i64, y as i64);
            let a = at(xi - 1, yi - 1);
            let b = at(xi, yi - 1);
            let c = at(xi + 1, yi - 1);
            let d = at(xi - 1, yi);
            let f = at(xi + 1, yi);
            let g = at(xi - 1, yi + 1);
            let hh = at(xi, yi + 1);
            let i = at(xi + 1, yi + 1);
            let dzdx = (c + 2.0 * f + i - (a + 2.0 * d + g)) / (8.0 * cell_meters);
            let dzdy = (g + 2.0 * hh + i - (a + 2.0 * b + c)) / (8.0 * cell_meters);
            let slope = (dzdx * dzdx + dzdy * dzdy).sqrt().atan();
            let aspect = dzdy.atan2(-dzdx);
            let mut hs = ZENITH_RAD.cos() * slope.cos()
                + ZENITH_RAD.sin() * slope.sin() * (AZIMUTH_RAD - aspect).cos();
            if hs < 0.0 {
                hs = 0.0;
            }
            let gray = js::round(hs * 255.0) as u8; // f64 → u8 saturates like Uint8ClampedArray
            // Flip rows so image row 0 = north.
            let o = ((h - 1 - y) * w + x) * 4;
            data[o] = gray;
            data[o + 1] = gray;
            data[o + 2] = gray;
            data[o + 3] = 255;
        }
    }

    Hillshade { data, w, h }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dims_downsample() {
        // 6400 src, MAX_EDGE 1024 → scale 7, w = floor(6400/7) = 914.
        let hs = build_hillshade_image(&vec![0f32; 8 * 8], 8, 8);
        assert_eq!((hs.w, hs.h), (8, 8)); // 8 < 1024 → scale 1
        assert_eq!(hs.data.len(), 8 * 8 * 4);
    }

    #[test]
    fn flat_grid_is_uniform_cos_zenith() {
        // A flat DEM → slope 0 → hs = cos(ZENITH) = cos(45°) ≈ 0.7071 → gray = round(180.31) = 180.
        let hs = build_hillshade_image(&vec![10.0f32; 16 * 16], 16, 16);
        for px in hs.data.chunks_exact(4) {
            assert_eq!(px[0], 180);
            assert_eq!(px[1], 180);
            assert_eq!(px[2], 180);
            assert_eq!(px[3], 255);
        }
    }

    #[test]
    fn constants_are_bit_identical_to_js() {
        // (315 * Math.PI) / 180 etc. — PI matches Math.PI, so these are exact.
        assert_eq!(AZIMUTH_RAD, (315.0 * core::f64::consts::PI) / 180.0);
        assert_eq!(ZENITH_RAD, core::f64::consts::PI / 2.0 - ALTITUDE_RAD);
    }
}
