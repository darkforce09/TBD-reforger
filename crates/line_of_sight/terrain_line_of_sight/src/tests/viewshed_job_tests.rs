//! **Role:** Unit tests of the sliced viewshed job: bit-identical to the one-call viewshed at every
//! budget, the same opening for an off-coverage observer, and cancellation mid-disc.
//! **Position:** Test-only child of the module it exercises,
//! [`crate::viewshed_job::ViewshedJob`] against
//! [`crate::viewshed::compute_viewshed`]; inputs from `viewshed_fixtures.rs`.
//! **Signals & state:** none; each test drives its own job with a counting clock.
//! **Invariants:** a finished job's raster equals the synchronous one cell for cell; a cancelled
//! job marches nothing more and keeps its partial raster.

use crate::viewshed::{ViewshedParams, compute_viewshed};
use crate::viewshed_fixtures::{flat_world, params};
use crate::viewshed_job::ViewshedJob;
use terrain_elevation::manifest::DemManifest;

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

#[test]
fn sliced_viewshed_is_bit_identical_to_the_sync_path() {
    let (m, p, elev) = sliced_fixture();
    let sync = compute_viewshed(&m, p, &elev);

    let (v, h, u) = sync.class_counts();
    assert!(
        v > 0 && h > 0 && u > 0,
        "fixture must produce visible/hidden/unknown: {v}/{h}/{u}"
    );
    for (i, budget) in [0.0, 3.0, 1e9].into_iter().enumerate() {
        let mut job = ViewshedJob::new(&m, p, 7).expect("under the cell cap");
        let batches = drain(&mut job, budget, &elev);
        let sliced = job.raster();
        assert_eq!(
            (sliced.cols, sliced.rows),
            (sync.cols, sync.rows),
            "budget {budget}: dims"
        );
        assert_eq!(
            (sliced.min_x, sliced.min_y, sliced.max_x, sliced.max_y),
            (sync.min_x, sync.min_y, sync.max_x, sync.max_y),
            "budget {budget}: world rect"
        );
        assert_eq!(
            sliced.cells, sync.cells,
            "budget {budget}: every cell must match the synchronous march ({batches} batches)"
        );
        let (rays, total) = job.progress();
        assert_eq!(rays, total, "budget {budget}: every ray marched");
        if i == 0 {
            assert_eq!(batches, total, "a zero budget slices to one ray per batch");
        }
    }
}

#[test]
fn sliced_viewshed_matches_the_off_coverage_opening() {
    let m = flat_world(600.0);
    let p = params(300.0, 300.0, None, 200.0, 8.0);
    let sync = compute_viewshed(&m, p, |_, _| Some(0.0));
    let job = ViewshedJob::new(&m, p, 1).expect("under the cell cap");
    assert!(job.done, "no honest eye — nothing to march");
    assert_eq!(job.raster().cells, sync.cells);
    assert_eq!(
        (job.raster().cols, job.raster().rows),
        (sync.cols, sync.rows)
    );
}

#[test]
fn viewshed_job_cancels_mid_disc() {
    let (m, p, elev) = sliced_fixture();
    let mut job = ViewshedJob::new(&m, p, 42).expect("under the cell cap");
    assert_eq!(job.generation, 42);
    let now = || 0.0f64;
    assert!(job.step(&elev, 0.0, &now), "one ray marched");
    let (before, total) = job.progress();
    assert!(before > 0 && before < total);
    let partial = job.raster().clone();
    job.cancel();
    assert!(job.done);
    assert!(
        !job.step(&elev, 1e9, &now),
        "a cancelled job marches nothing"
    );
    assert_eq!(
        job.progress().0,
        before,
        "cursor frozen at the cancel point"
    );
    assert_eq!(
        job.raster().cells,
        partial.cells,
        "partial raster preserved"
    );

    assert_eq!(partial.cells.len(), partial.cols * partial.rows);
}
