//! T-090.11.3 rotation-order pin — which Euler composition does Enfusion use for `angles`
//! (pitch about X, yaw about Y, roll about Z)?
//!
//! The Workbench recon of a *tilted parent with a rotated child* is an exact, compile-free
//! observable: the child's `relPos` (world-axis offset from the parent origin) is
//! `M(parent) · localCoords`, and the child's world yaw (the engine's own `GetAngles()[1]` of
//! `M(parent) · M(child)`) shifts away from the parent's yaw by an amount that depends on the
//! composition order at first order in the parent's roll. Every hypothesis — 6 axis orders ×
//! 8 sign patterns — is scored against both; the winner must be `Rigid::from_enfusion`
//! ([`RIGID_HYPOTHESIS`]: `R_y(yaw) · R_x(-pitch) · R_z(-roll)`) and unique by a clear margin.
//!
//! `cargo xtask map rotation-pin --fixture <json>` prints the ranked table for any sample
//! (`xtask/tests/fixtures/rotation_pin_*.json`, captured from `recon` + the prefab text).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinFixture {
    #[serde(default)]
    pub source: String,
    pub parent: PinParent,
    pub child: PinChild,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinParent {
    #[serde(default)]
    pub prefab: String,
    pub angles_deg: [f64; 3],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinChild {
    #[serde(default)]
    pub prefab: String,
    pub local_coords: [f64; 3],
    pub local_angles_deg: [f64; 3],
    pub observed_rel_pos: [f64; 3],
    pub observed_yaw_deg: f64,
}

pub type Mat3 = [[f64; 3]; 3];

fn rot(axis: usize, deg: f64) -> Mat3 {
    let (s, c) = deg.to_radians().sin_cos();
    match axis {
        0 => [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]],
        1 => [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]],
        _ => [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
    }
}

pub fn mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

fn apply(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (0..3).map(|k| m[i][k] * v[k]).sum();
    }
    out
}

/// One composition hypothesis: `R_order[0](s0·a0) · R_order[1](s1·a1) · R_order[2](s2·a2)`
/// where the angle for axis `k` is `angles[k]` (x = pitch, y = yaw, z = roll).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hypothesis {
    pub order: [usize; 3],
    pub signs: [f64; 3],
}

impl Hypothesis {
    pub fn all() -> Vec<Hypothesis> {
        let orders = [
            [1, 0, 2],
            [1, 2, 0],
            [0, 1, 2],
            [0, 2, 1],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let mut out = Vec::new();
        for order in orders {
            for bits in 0..8u8 {
                let sign = |b: u8| if bits & b == 0 { 1.0 } else { -1.0 };
                out.push(Hypothesis {
                    order,
                    signs: [sign(1), sign(2), sign(4)],
                });
            }
        }
        out
    }

    pub fn name(&self) -> String {
        let ax = |a: usize| ["X", "Y", "Z"][a];
        let sg = |a: usize| if self.signs[a] > 0.0 { "+" } else { "-" };
        format!(
            "{}·{}·{} (pitch{} yaw{} roll{})",
            ax(self.order[0]),
            ax(self.order[1]),
            ax(self.order[2]),
            sg(0),
            sg(1),
            sg(2)
        )
    }

    pub fn matrix(&self, angles_deg: [f64; 3]) -> Mat3 {
        let mut m = rot(
            self.order[0],
            self.signs[self.order[0]] * angles_deg[self.order[0]],
        );
        for &a in &self.order[1..] {
            m = mul(&m, &rot(a, self.signs[a] * angles_deg[a]));
        }
        m
    }

    /// Recover `angles` with `matrix(angles) ≈ m` under this hypothesis: coarse grid + coordinate
    /// refinement, middle angle kept in [-90°, 90°] (the asin branch every engine reports).
    pub fn decompose(&self, m: &Mat3) -> [f64; 3] {
        let err = |a: [f64; 3]| {
            let p = self.matrix(a);
            let mut e = 0.0;
            for i in 0..3 {
                for j in 0..3 {
                    e += (p[i][j] - m[i][j]).powi(2);
                }
            }
            e
        };
        let mid = self.order[1];
        let mut best = ([0.0; 3], f64::INFINITY);
        let mut i0 = -180.0;
        while i0 < 180.0 {
            let mut i1 = -90.0;
            while i1 <= 90.0 {
                let mut i2 = -180.0;
                while i2 < 180.0 {
                    let mut a = [0.0; 3];
                    a[self.order[0]] = i0;
                    a[mid] = i1;
                    a[self.order[2]] = i2;
                    let e = err(a);
                    if e < best.1 {
                        best = (a, e);
                    }
                    i2 += 10.0;
                }
                i1 += 10.0;
            }
            i0 += 10.0;
        }
        let mut a = best.0;
        let mut step = 5.0;
        while step > 1e-7 {
            let mut improved = false;
            for k in 0..3 {
                for dir in [-1.0, 1.0] {
                    let mut t = a;
                    t[k] += dir * step;
                    if k == mid {
                        t[k] = t[k].clamp(-90.0, 90.0);
                    }
                    if err(t) < err(a) {
                        a = t;
                        improved = true;
                    }
                }
            }
            if !improved {
                step *= 0.5;
            }
        }
        for v in &mut a {
            *v = wrap(*v);
        }
        a
    }
}

fn wrap(d: f64) -> f64 {
    let mut x = d % 360.0;
    if x > 180.0 {
        x -= 360.0;
    }
    if x <= -180.0 {
        x += 360.0;
    }
    x
}

#[derive(Debug, Clone)]
pub struct Score {
    pub hypothesis: Hypothesis,
    pub predicted_rel: [f64; 3],
    pub rel_err_m: f64,
    pub predicted_yaw_deg: f64,
    pub yaw_err_deg: f64,
}

impl Score {
    /// Combined residual: metres plus degrees scaled to metres at a 1 m lever arm.
    pub fn total(&self) -> f64 {
        self.rel_err_m + self.yaw_err_deg.to_radians()
    }
}

pub fn score_all(fx: &PinFixture) -> Vec<Score> {
    let mut out: Vec<Score> = Hypothesis::all()
        .into_iter()
        .map(|h| {
            let mp = h.matrix(fx.parent.angles_deg);
            let mc = h.matrix(fx.child.local_angles_deg);
            let rel = apply(&mp, fx.child.local_coords);
            let rel_err = (0..3)
                .map(|i| (rel[i] - fx.child.observed_rel_pos[i]).powi(2))
                .sum::<f64>()
                .sqrt();
            let yaw = h.decompose(&mul(&mp, &mc))[1];
            Score {
                hypothesis: h,
                predicted_rel: rel,
                rel_err_m: rel_err,
                predicted_yaw_deg: yaw,
                yaw_err_deg: wrap(yaw - fx.child.observed_yaw_deg).abs(),
            }
        })
        .collect();
    out.sort_by(|a, b| a.total().total_cmp(&b.total()));
    out
}

/// The hypothesis `Rigid::from_enfusion` implements — Enfusion's convention as pinned by the
/// garbage-container sample (2026-09-03): `R_y(+yaw) · R_x(-pitch) · R_z(-roll)`. Yaw agrees
/// with the right-handed `R_y` (the 88-socket farmhouse recon already showed that); pitch and
/// roll enter negated, i.e. positive pitch is nose-down and positive roll is right-side-down
/// in the engine's left-handed X-right / Y-up / Z-forward frame.
pub const RIGID_HYPOTHESIS: Hypothesis = Hypothesis {
    order: [1, 0, 2],
    signs: [-1.0, 1.0, -1.0],
};

pub fn load_fixture(path: &Path) -> Result<PinFixture> {
    serde_json::from_str(&fs::read_to_string(path).with_context(|| path.display().to_string())?)
        .context("parse rotation-pin fixture")
}

pub fn run_rotation_pin(args: &[String]) -> Result<u8> {
    let mut fixture = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--fixture" && i + 1 < args.len() {
            fixture = Some(args[i + 1].clone());
            i += 2;
        } else {
            eprintln!(
                "rotation-pin: unknown arg {} (usage: --fixture <json>)",
                args[i]
            );
            return Ok(1);
        }
    }
    let path = fixture.context("--fixture <json> is required")?;
    let fx = load_fixture(Path::new(&path))?;
    let scores = score_all(&fx);
    println!(
        "rotation-pin {} — parent {} angles {:?} · child {} local {:?} @ {:?} · observed rel {:?} yaw {}",
        fx.source,
        fx.parent.prefab,
        fx.parent.angles_deg,
        fx.child.prefab,
        fx.child.local_angles_deg,
        fx.child.local_coords,
        fx.child.observed_rel_pos,
        fx.child.observed_yaw_deg
    );
    for (rank, s) in scores.iter().enumerate().take(12) {
        println!(
            "  #{:<2} {:<34} rel {:?} (err {:.4} m) · yaw {:>9.4}° (err {:.4}°) · total {:.5}{}",
            rank + 1,
            s.hypothesis.name(),
            s.predicted_rel.map(|v| (v * 1e4).round() / 1e4),
            s.rel_err_m,
            s.predicted_yaw_deg,
            s.yaw_err_deg,
            s.total(),
            if s.hypothesis == RIGID_HYPOTHESIS {
                "  ← Rigid::from_enfusion"
            } else {
                ""
            }
        );
    }
    let winner = &scores[0];
    let ok = winner.hypothesis == RIGID_HYPOTHESIS;
    println!(
        "winner: {} · margin to runner-up {:.5} · Rigid::from_enfusion {}",
        winner.hypothesis.name(),
        scores[1].total() - winner.total(),
        if ok { "CONFIRMED" } else { "REJECTED" }
    );
    Ok(u8::from(!ok))
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_engine_core::geometry::rigid::Rigid;

    #[test]
    fn rigid_from_enfusion_is_the_y_x_z_hypothesis() {
        let angles = [14.991, 90.532, -90.279];
        let m = RIGID_HYPOTHESIS.matrix(angles);
        let r = Rigid::from_enfusion([0.0; 3], angles, 1.0);
        for i in 0..3 {
            for j in 0..3 {
                assert!((m[i][j] - r.m[i][j]).abs() < 1e-12, "{i}{j}");
            }
        }
        let back = RIGID_HYPOTHESIS.decompose(&m);
        for k in 0..3 {
            assert!((back[k] - angles[k]).abs() < 1e-5, "{back:?}");
        }
        assert_eq!(Hypothesis::all().len(), 48);
    }

    /// The pin: GarbageContainer_01 (tilted 3.0°/4.75°) with its lid child (pitch -55°) as
    /// recorded by the Workbench recon on 2026-09-03.
    #[test]
    fn garbage_container_lid_pins_y_x_z_with_negated_pitch_and_roll() {
        let root = crate::root::find_repo_root().unwrap();
        let fx =
            load_fixture(&root.join("xtask/tests/fixtures/rotation_pin_GarbageContainer_01.json"))
                .unwrap();
        let scores = score_all(&fx);
        let winner = &scores[0];
        assert_eq!(
            winner.hypothesis,
            RIGID_HYPOTHESIS,
            "winner {} (rel {:.4} m, yaw err {:.4}°)",
            winner.hypothesis.name(),
            winner.rel_err_m,
            winner.yaw_err_deg
        );
        assert!(
            winner.rel_err_m < 0.005,
            "rel err {:.4} m",
            winner.rel_err_m
        );
        assert!(
            winner.yaw_err_deg < 0.05,
            "yaw err {:.4}°",
            winner.yaw_err_deg
        );
        let runner = &scores[1];
        assert!(
            runner.total() > 4.0 * winner.total().max(0.002),
            "no clear margin: {} total {:.5} vs {} total {:.5}",
            winner.hypothesis.name(),
            winner.total(),
            runner.hypothesis.name(),
            runner.total()
        );
    }
}
