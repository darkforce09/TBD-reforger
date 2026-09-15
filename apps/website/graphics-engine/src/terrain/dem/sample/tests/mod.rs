//! Role: Module boundary for terrain/dem/sample/tests.
//! Position: `terrain/dem/sample/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

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

fn synth(span: f64, w: usize, h: usize) -> DemManifest {
    DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: span,
        max_y: span,
        width_px: w,
        height_px: h,
        flip_x: false,
        flip_z: false,
        height_min_m: 0.0,
        height_max_m: 100.0,
    }
}

fn flat_world(span: f64) -> DemManifest {
    DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: span,
        max_y: span,
        width_px: 1,
        height_px: 1,
        flip_x: false,
        flip_z: false,
        height_min_m: 0.0,
        height_max_m: 500.0,
    }
}

fn params(obs_x: f64, obs_y: f64, ground: Option<f64>, radius: f64, cell: f64) -> ViewshedParams {
    ViewshedParams {
        obs_x,
        obs_y,
        observer_ground_m: ground,
        eye_height_m: 1.8,
        radius_m: radius,
        cell_m: cell,
    }
}

fn disc_counts(vs: &Viewshed, radius: f64) -> (usize, usize, usize) {
    let (mut v, mut h, mut u) = (0usize, 0usize, 0usize);
    for r in 0..vs.rows {
        for c in 0..vs.cols {
            let x = vs.min_x + c as f64 * 8.0;
            let y = vs.min_y + r as f64 * 8.0;
            if ((x - vs.obs_x).powi(2) + (y - vs.obs_y).powi(2)).sqrt() > radius {
                continue;
            }
            match vs.at(c, r) {
                Visibility::Visible => v += 1,
                Visibility::Hidden => h += 1,
                Visibility::Unknown => u += 1,
            }
        }
    }
    (v, h, u)
}

fn sliced_fixture() -> (
    DemManifest,
    ViewshedParams,
    impl Fn(f64, f64) -> Option<f64>,
) {
    let m = flat_world(600.0);

    let p = params(120.0, 140.0, Some(10.0), 300.0, 8.0);
    let elev = |x: f64, y: f64| -> Option<f64> {
        if (x - 300.0).abs() < 12.0 && (y - 300.0).abs() < 12.0 {
            return None;
        }

        let ridge = if (x - 220.0).abs() < 8.0 { 60.0 } else { 0.0 };
        Some(10.0 + ridge + y * 0.02)
    };
    (m, p, elev)
}

fn drain(job: &mut ViewshedJob, budget_ms: f64, elev: &dyn Fn(f64, f64) -> Option<f64>) -> usize {
    let t = std::cell::Cell::new(0.0f64);
    let now = || {
        t.set(t.get() + 1.0);
        t.get()
    };
    let mut batches = 0usize;
    while !job.done {
        assert!(job.step(elev, budget_ms, &now), "a live job must march");
        batches += 1;
        assert!(batches < 100_000, "job never finished");
    }
    batches
}

mod cases_1;
