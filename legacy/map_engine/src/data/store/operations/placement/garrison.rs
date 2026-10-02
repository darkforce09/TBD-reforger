//! Role: garrison.
//! Position: `doc/operations/placement` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Pt;

/// A firing position on a building perimeter: the world point plus the outward-facing yaw (degrees clockwise from north) an occupant would face — out through the nearest wall.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiringPosition {
    /// Pos.
    pub pos: Pt,

    /// Yaw deg.
    pub yaw_deg: f64,
}

/// Garrison firing positions for one building OBB, capped at `count` (the group size). The building is given by its centre `(cx, cy)`, half-extents `(half_x, half_y)` and `rotation_deg` (the same OBB parameterisation `world::obb::obb_corners` uses — `0° = north (+y)`, clockwise-positive).
#[allow(dead_code)]
#[must_use]
pub fn garrison_firing_positions(
    cx: f64,
    cy: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
    count: usize,
) -> Vec<FiringPosition> {
    let inset = 1.0;
    let hx = half_x - inset;
    let hy = half_y - inset;
    if count == 0 || hx <= 0.0 || hy <= 0.0 {
        return Vec::new();
    }
    let rad = rotation_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();

    let to_world = |dx: f64, dy: f64| Pt::new(cx + dx * cos + dy * sin, cy - dx * sin + dy * cos);

    let perim = 2.0 * (2.0 * hx + 2.0 * hy);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let s = perim * (i as f64) / (count as f64);
        let (dx, dy, out_normal_local) = perimeter_point(hx, hy, s);
        let world = to_world(dx, dy);

        let local_bearing = (out_normal_local.0).atan2(out_normal_local.1).to_degrees();
        let yaw = (rotation_deg + local_bearing).rem_euclid(360.0);
        out.push(FiringPosition {
            pos: world,
            yaw_deg: yaw,
        });
    }
    out
}

/// `allow(dead_code)`: only [`garrison_firing_positions`] calls it — same scope-discovery status.
#[allow(dead_code)]
pub fn perimeter_point(hx: f64, hy: f64, s: f64) -> (f64, f64, (f64, f64)) {
    let top = 2.0 * hx;
    let right = 2.0 * hy;
    let bottom = 2.0 * hx;
    let mut r = s;
    if r < top {
        return (-hx + r, hy, (0.0, 1.0));
    }
    r -= top;
    if r < right {
        return (hx, hy - r, (1.0, 0.0));
    }
    r -= right;
    if r < bottom {
        return (hx - r, -hy, (0.0, -1.0));
    }
    r -= bottom;
    (-hx, -hy + r, (-1.0, 0.0))
}
