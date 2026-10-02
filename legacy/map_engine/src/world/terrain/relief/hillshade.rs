//! Role: hillshade.
//! Position: `world/terrain/relief` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use core::f64::consts::PI;

const MAX_EDGE: usize = 1024;
const ALTITUDE_RAD: f64 = (45.0 * PI) / 180.0;
const AZIMUTH_RAD: f64 = (315.0 * PI) / 180.0;
const ZENITH_RAD: f64 = PI / 2.0 - ALTITUDE_RAD;

/// Row-flipped RGBA image + its dims. The JS wraps `data` in an `ImageData(w, h)`.
#[derive(Clone, Debug)]
pub struct Hillshade {
    /// Data.
    pub data: Vec<u8>,

    /// W.
    pub w: usize,

    /// H.
    pub h: usize,
}

/// Build the Horn hillshade RGBA image. Mirror of `buildHillshadeImage` (`useDemLayer.ts:28`).
#[must_use]
pub fn build_hillshade_image(meters: &[f32], src_w: usize, src_h: usize) -> Hillshade {
    let scale = 1.max((src_w.max(src_h) as f64 / MAX_EDGE as f64).ceil() as usize);
    let w = 1.max(src_w / scale);
    let h = 1.max(src_h / scale);
    let cell_meters = src_w as f64 / w as f64;

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
            let gray = crate::camera::math::shaping::round(hs * 255.0) as u8;

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
#[path = "tests/hillshade_tests.rs"]
mod tests;
