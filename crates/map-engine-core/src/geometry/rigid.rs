//! Rigid (rotation + translation, optional uniform scale) transforms in the building /
//! model frame — the TLAS side of T-090.11: a prop's BLAS is raycast in its own space by
//! mapping the segment through the instance's inverse, and its hits come back through the
//! forward map. `t` along the segment is invariant under a rigid map, so hits merge with
//! the shell's without conversion.
//!
//! Conventions (Enfusion, left-handed, y up): a quaternion is `[x, y, z, w]`; Euler angles
//! are `[pitch, yaw, roll]` degrees about the local X, Y, Z axes, composed as
//! `R = R_y(yaw) · R_x(pitch) · R_z(roll)` (roll first, yaw last) — the order the recon
//! oracle test in T-090.11.3 pins. Doors rotate about their leaf's local Y ([`Rigid::rot_y`]).

/// 3×3 rotation (row-major, applied as `m · v`) plus translation; `scale` is a uniform
/// factor applied before rotation (`p' = m · (scale · p) + t`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid {
    pub m: [[f64; 3]; 3],
    pub t: [f64; 3],
    pub scale: f64,
}

impl Default for Rigid {
    fn default() -> Self {
        Self::identity()
    }
}

impl Rigid {
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            t: [0.0; 3],
            scale: 1.0,
        }
    }

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
            .compose(&Self::rot_x(angles_deg[0]))
            .compose(&Self::rot_z(angles_deg[2]));
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
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3], eps: f64) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() <= eps)
    }

    #[test]
    fn rot_y_turns_x_toward_z_and_inverse_undoes() {
        let r = Rigid::rot_y(90.0);
        assert!(close(r.dir([1.0, 0.0, 0.0]), [0.0, 0.0, -1.0], 1e-12));
        assert!(close(r.dir([0.0, 0.0, 1.0]), [1.0, 0.0, 0.0], 1e-12));
        let t = Rigid::from_enfusion([3.0, 1.0, -2.0], [10.0, -35.0, 5.0], 1.25);
        let p = [7.5, -2.0, 11.0];
        let back = t.inverse().point(t.point(p));
        assert!(close(back, p, 1e-12), "{back:?}");
        let id = t.compose(&t.inverse());
        assert!(close(id.t, [0.0; 3], 1e-12) && (id.scale - 1.0).abs() < 1e-12);
        for i in 0..3 {
            for j in 0..3 {
                let want = if i == j { 1.0 } else { 0.0 };
                assert!((id.m[i][j] - want).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn quaternion_round_trips_and_matches_euler_axes() {
        let s = 0.5f64.sqrt();
        let q = Rigid::from_quat_pos([0.0, s, 0.0, s], [1.0, 2.0, 3.0]);
        let e = Rigid::from_enfusion([1.0, 2.0, 3.0], [0.0, 90.0, 0.0], 1.0);
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (q.m[i][j] - e.m[i][j]).abs() < 1e-12,
                    "yaw 90 via quat == euler"
                );
            }
        }
        let back = q.to_quat();
        assert!(
            close([back[0], back[1], back[2]], [0.0, s, 0.0], 1e-12) && (back[3] - s).abs() < 1e-12
        );
        assert!((q.yaw_deg() - 90.0).abs() < 1e-9);
        // A general rotation survives matrix → quaternion → matrix.
        let g = Rigid::from_enfusion([0.0; 3], [88.816, -180.0, 96.7], 1.0);
        let g2 = Rigid::from_quat_pos(g.to_quat(), [0.0; 3]);
        for i in 0..3 {
            for j in 0..3 {
                assert!((g.m[i][j] - g2.m[i][j]).abs() < 1e-9);
            }
        }
        // Composition order: yaw applied last.
        let ypr = Rigid::from_enfusion([0.0; 3], [30.0, 40.0, 50.0], 1.0);
        let manual = Rigid::rot_y(40.0)
            .compose(&Rigid::rot_x(30.0))
            .compose(&Rigid::rot_z(50.0));
        for i in 0..3 {
            for j in 0..3 {
                assert!((ypr.m[i][j] - manual.m[i][j]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn nested_composition_keeps_sub_micrometre_precision() {
        // world → building → prop → hit → back, at building scale.
        let building = Rigid::from_enfusion([6400.0, 51.2, 6410.5], [0.0, 137.5, 0.0], 1.0);
        let prop = Rigid::from_enfusion([-8.87, 3.58, -5.29], [88.816, -180.0, 96.7], 1.152);
        let world_to_prop = building.compose(&prop).inverse();
        let p_world = [6412.345, 55.5, 6398.25];
        let p_prop = world_to_prop.point(p_world);
        let back = building.compose(&prop).point(p_prop);
        assert!(close(back, p_world, 1e-6), "{back:?} vs {p_world:?}");
        let (lo, hi) = prop.aabb_of([-0.5, 0.0, -0.5], [0.5, 0.8, 0.5]);
        assert!(lo.iter().zip(hi.iter()).all(|(a, b)| a < b));
    }
}
