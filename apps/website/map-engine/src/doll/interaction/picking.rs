//! Role: picking.
//! Position: `doll/interaction` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::math::glmat4::invert;
use crate::camera::math::glmat4::transform_vector;
use crate::camera::orbit::projection::view_proj_gl;
use crate::doll::scene::instances::instances;

/// Representative world point for a region's callout: the first instance's model translation (the part's center).
#[must_use]
pub fn anchor_world(region: i32) -> Option<[f64; 3]> {
    instances()
        .into_iter()
        .find(|i| i.region == region)
        .map(|i| [i.model[12], i.model[13], i.model[14]])
}

/// The region anchor projected to CSS pixels, or `None` when the region is unknown or the point sits behind the camera (w ≤ 0) — the callout hides then.
#[must_use]
pub fn anchor_px(yaw: f64, w_px: f64, h_px: f64, region: i32) -> Option<(f64, f64)> {
    if w_px <= 0.0 || h_px <= 0.0 {
        return None;
    }
    let p = anchor_world(region)?;
    let m = view_proj_gl(yaw, w_px, h_px);
    let cx = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
    let cy = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
    let cw = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
    if cw <= 0.0 {
        return None;
    }
    let ndc_x = cx / cw;
    let ndc_y = cy / cw;
    Some((((ndc_x + 1.0) / 2.0) * w_px, ((1.0 - ndc_y) / 2.0) * h_px))
}

/// Ray unit box.
pub(crate) fn ray_unit_box(origin: [f64; 3], dir: [f64; 3]) -> Option<f64> {
    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;
    for a in 0..3 {
        if dir[a].abs() < 1e-12 {
            if origin[a].abs() > 0.5 {
                return None;
            }
            continue;
        }
        let inv = 1.0 / dir[a];
        let t0 = (-0.5 - origin[a]) * inv;
        let t1 = (0.5 - origin[a]) * inv;
        let (lo, hi) = if t0 < t1 { (t0, t1) } else { (t1, t0) };
        t_min = t_min.max(lo);
        t_max = t_max.min(hi);
        if t_min > t_max {
            return None;
        }
    }
    if t_max < 0.0 {
        None
    } else {
        Some(t_min.max(0.0))
    }
}

/// Pick the nearest clickable region under a device pixel, or -1. Pure — callable from vitest through the wasm shim without a GPU.
#[must_use]
pub fn pick(yaw: f64, w_px: f64, h_px: f64, x_px: f64, y_px: f64) -> i32 {
    if w_px <= 0.0 || h_px <= 0.0 {
        return -1;
    }
    let Some(inv_vp) = invert(&view_proj_gl(yaw, w_px, h_px)) else {
        return -1;
    };
    let ndc_x = (x_px / w_px) * 2.0 - 1.0;
    let ndc_y = 1.0 - (y_px / h_px) * 2.0;
    let near = transform_vector(&inv_vp, [ndc_x, ndc_y, -1.0, 1.0]);
    let far = transform_vector(&inv_vp, [ndc_x, ndc_y, 1.0, 1.0]);
    let origin = [near[0], near[1], near[2]];
    let dir = [far[0] - near[0], far[1] - near[1], far[2] - near[2]];

    let mut best: Option<(f64, i32)> = None;
    for inst in instances() {
        if inst.region < 0 {
            continue;
        }
        let Some(inv_model) = invert(&inst.model) else {
            continue;
        };
        let o = transform_vector(&inv_model, [origin[0], origin[1], origin[2], 1.0]);
        let d = {
            let m = &inv_model;
            [
                m[0] * dir[0] + m[4] * dir[1] + m[8] * dir[2],
                m[1] * dir[0] + m[5] * dir[1] + m[9] * dir[2],
                m[2] * dir[0] + m[6] * dir[1] + m[10] * dir[2],
            ]
        };
        if let Some(t) = ray_unit_box([o[0], o[1], o[2]], d)
            && best.is_none_or(|(bt, _)| t < bt)
        {
            best = Some((t, inst.region));
        }
    }
    best.map_or(-1, |(_, r)| r)
}
