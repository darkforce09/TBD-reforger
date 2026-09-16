//! Role: transform.
//! Position: `world/architecture/compound` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Rigid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid {
    /// M.
    pub m: [[f64; 3]; 3],

    /// T.
    pub t: [f64; 3],

    /// Scale.
    pub scale: f64,
}

impl Default for Rigid {
    fn default() -> Self {
        Self::identity()
    }
}

impl Rigid {
    /// Identity.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            t: [0.0; 3],
            scale: 1.0,
        }
    }

    /// Translation.
    #[must_use]
    pub fn translation(t: [f64; 3]) -> Self {
        Self {
            t,
            ..Self::identity()
        }
    }

    /// Rotation about +Y by `deg` (the door hinge axis), no translation.
    #[must_use]
    pub fn rot_y(deg: f64) -> Self {
        let (s, c) = deg.to_radians().sin_cos();
        Self {
            m: [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]],
            ..Self::identity()
        }
    }

    /// Rotation about +X by `deg`.
    #[must_use]
    pub fn rot_x(deg: f64) -> Self {
        let (s, c) = deg.to_radians().sin_cos();
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]],
            ..Self::identity()
        }
    }

    /// Rotation about +Z by `deg`.
    #[must_use]
    pub fn rot_z(deg: f64) -> Self {
        let (s, c) = deg.to_radians().sin_cos();
        Self {
            m: [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
            ..Self::identity()
        }
    }

    /// From a unit quaternion `[x, y, z, w]` (normalized here) and a position.
    #[must_use]
    pub fn from_quat_pos(q: [f64; 4], pos: [f64; 3]) -> Self {
        let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        let (x, y, z, w) = if n > 0.0 {
            (q[0] / n, q[1] / n, q[2] / n, q[3] / n)
        } else {
            (0.0, 0.0, 0.0, 1.0)
        };
        let m = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ];
        Self {
            m,
            t: pos,
            scale: 1.0,
        }
    }

    /// From Enfusion `coords` + `angles [pitch, yaw, roll]` (degrees) + uniform `scale`.
    #[must_use]
    pub fn from_enfusion(pos: [f64; 3], angles_deg: [f64; 3], scale: f64) -> Self {
        let r = Self::rot_y(angles_deg[1])
            .compose(&Self::rot_x(-angles_deg[0]))
            .compose(&Self::rot_z(-angles_deg[2]));
        Self {
            m: r.m,
            t: pos,
            scale,
        }
    }

    /// `self ∘ other`: apply `other` first, then `self` (`p' = self(other(p))`).
    #[must_use]
    pub fn compose(&self, other: &Rigid) -> Self {
        let mut m = [[0.0; 3]; 3];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = self.m[i][0] * other.m[0][j]
                    + self.m[i][1] * other.m[1][j]
                    + self.m[i][2] * other.m[2][j];
            }
        }
        Self {
            m,
            t: self.point(other.t),
            scale: self.scale * other.scale,
        }
    }

    /// Inverse (rotation transposed, scale reciprocal).
    #[must_use]
    pub fn inverse(&self) -> Self {
        let mt = [
            [self.m[0][0], self.m[1][0], self.m[2][0]],
            [self.m[0][1], self.m[1][1], self.m[2][1]],
            [self.m[0][2], self.m[1][2], self.m[2][2]],
        ];
        let inv_s = 1.0 / self.scale;
        let neg = [-self.t[0], -self.t[1], -self.t[2]];
        let r = Self {
            m: mt,
            t: [0.0; 3],
            scale: inv_s,
        };
        let t = r.point(neg);
        Self {
            m: mt,
            t,
            scale: inv_s,
        }
    }

    /// Transform a point.
    #[must_use]
    pub fn point(&self, p: [f64; 3]) -> [f64; 3] {
        let s = self.scale;
        let v = [p[0] * s, p[1] * s, p[2] * s];
        [
            self.m[0][0] * v[0] + self.m[0][1] * v[1] + self.m[0][2] * v[2] + self.t[0],
            self.m[1][0] * v[0] + self.m[1][1] * v[1] + self.m[1][2] * v[2] + self.t[1],
            self.m[2][0] * v[0] + self.m[2][1] * v[1] + self.m[2][2] * v[2] + self.t[2],
        ]
    }

    /// Transform a direction (rotation only, no translation, no scale).
    #[must_use]
    pub fn dir(&self, d: [f64; 3]) -> [f64; 3] {
        [
            self.m[0][0] * d[0] + self.m[0][1] * d[1] + self.m[0][2] * d[2],
            self.m[1][0] * d[0] + self.m[1][1] * d[1] + self.m[1][2] * d[2],
            self.m[2][0] * d[0] + self.m[2][1] * d[1] + self.m[2][2] * d[2],
        ]
    }

    /// Rotation as a unit quaternion `[x, y, z, w]` (w ≥ 0).
    #[must_use]
    pub fn to_quat(&self) -> [f64; 4] {
        let m = &self.m;
        let tr = m[0][0] + m[1][1] + m[2][2];
        let q = if tr > 0.0 {
            let s = (tr + 1.0).sqrt() * 2.0;
            [
                (m[2][1] - m[1][2]) / s,
                (m[0][2] - m[2][0]) / s,
                (m[1][0] - m[0][1]) / s,
                0.25 * s,
            ]
        } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
            let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
            [
                0.25 * s,
                (m[0][1] + m[1][0]) / s,
                (m[0][2] + m[2][0]) / s,
                (m[2][1] - m[1][2]) / s,
            ]
        } else if m[1][1] > m[2][2] {
            let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
            [
                (m[0][1] + m[1][0]) / s,
                0.25 * s,
                (m[1][2] + m[2][1]) / s,
                (m[0][2] - m[2][0]) / s,
            ]
        } else {
            let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
            [
                (m[0][2] + m[2][0]) / s,
                (m[1][2] + m[2][1]) / s,
                0.25 * s,
                (m[1][0] - m[0][1]) / s,
            ]
        };
        if q[3] < 0.0 {
            [-q[0], -q[1], -q[2], -q[3]]
        } else {
            q
        }
    }

    /// Yaw about +Y in degrees, `(-180, 180]` — the plan-view heading of the transform.
    #[must_use]
    pub fn yaw_deg(&self) -> f64 {
        let f = self.dir([0.0, 0.0, 1.0]);
        f[0].atan2(f[2]).to_degrees()
    }

    /// Axis-aligned bounds of a transformed box (`min`..`max` in the source frame).
    #[must_use]
    pub fn aabb_of(&self, min: [f64; 3], max: [f64; 3]) -> ([f64; 3], [f64; 3]) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for corner in 0..8u32 {
            let p = self.point([
                if corner & 1 != 0 { max[0] } else { min[0] },
                if corner & 2 != 0 { max[1] } else { min[1] },
                if corner & 4 != 0 { max[2] } else { min[2] },
            ]);
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        (lo, hi)
    }
}

#[cfg(test)]
#[path = "tests/transform_tests.rs"]
mod tests;
