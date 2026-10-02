//! Role: the scheduler's lane isolation, budget slicing, cap refusal, and decline-without-a-sampler.
//! Position: `editing/tools/viewshed_scheduler/tests` in the map engine.
//! Signals & state: the scheduler's own thread-local slots.
//! Invariants: the budget is proven by inequality against a deliberately slow cell test, never by a
//! wall-clock deadline.

use super::*;
use crate::spatial::los::interior::wash::MAX_WASH_RADIUS_M;
use crate::spatial::los::interior::wash::WASH_BATCH_CELLS;
use crate::spatial::los::interior::wash::WashParams;
use crate::spatial::los::interior::wash::wash_band;
use crate::spatial::los::terrain::viewshed::Visibility;

/// A blocker that hides everything east of the observer (so a wash has all three classes) and
/// costs ~100 µs a call. The COST is the point: the scheduler's budget is only observable when a
/// cell costs something, and at 100 µs one [`WASH_BATCH_CELLS`] batch is ~25 ms — six times
/// [`VIEWSHED_BUDGET_MS`], so a step provably cannot run away with the frame. The margin is
/// deliberately huge: the assertions below are inequalities, never a wall-clock deadline.
fn slow_blocked(a: [f64; 3], b: [f64; 3]) -> bool {
    let t = std::time::Instant::now();
    while t.elapsed().as_micros() < 100 {
        std::hint::spin_loop();
    }
    b[0] > a[0] + 1.0
}

fn params() -> WashParams {
    WashParams {
        radius_m: 4.0,
        ..WashParams::default()
    }
}

/// ONE ACTIVE JOB PER TOOL, and a submit cancels only its own tool: the wash lane survives a
/// terrain cancel, and a second wash submit retires the first (a fresh generation, cursor back
/// to zero).
#[test]
fn one_active_job_per_tool_and_a_submit_cancels_its_own() {
    let p = params();
    let first = submit_wash(0, 1.0, [0.0, 1.0, 0.0], &p).expect("under the cap");
    assert_eq!(
        active_generation(ViewshedTool::BuildingWash),
        Some(first),
        "the wash lane holds the job it was handed"
    );
    // A submit does NO work: the job is live and untouched until someone steps it.
    assert_eq!(progress(ViewshedTool::BuildingWash), Some((0, 32 * 32)));
    // One step stops INSIDE the budget — it decides whole batches and hands the frame back
    // rather than finishing a wash that costs ~80 ms of `slow_blocked`.
    assert_eq!(step_wash(&slow_blocked), Some(false), "budget respected");
    let (done, total) = progress(ViewshedTool::BuildingWash).expect("live");
    assert!(
        done > 0 && done < total,
        "stepped but not finished: {done}/{total}"
    );
    assert_eq!(done % WASH_BATCH_CELLS, 0, "whole batches only");
    // The terrain lane is idle and cancelling it must not touch the wash.
    assert!(!cancel(ViewshedTool::Terrain));
    assert_eq!(active_generation(ViewshedTool::BuildingWash), Some(first));
    // A second submit replaces the first: newer stamp, cursor rewound.
    let second = submit_wash(0, 1.0, [0.0, 1.0, 0.0], &p).expect("under the cap");
    assert!(
        second > first,
        "generation is monotonic: {first} -> {second}"
    );
    assert_eq!(progress(ViewshedTool::BuildingWash), Some((0, 32 * 32)));
    assert!(cancel(ViewshedTool::BuildingWash));
    assert_eq!(active_generation(ViewshedTool::BuildingWash), None);
    assert!(take_wash().is_none());
}

/// Driven to completion through the scheduler, the wash equals the synchronous `wash_band` —
/// the slicing is invisible in the result.
#[test]
fn a_scheduled_wash_finishes_equal_to_the_sync_path() {
    let p = params();
    let obs = [0.0, 1.0, 0.0];
    let sync = wash_band(0, 1.0, obs, &p, slow_blocked);
    let (v, h, u) = sync.class_counts();
    assert!(v > 0 && h > 0 && u > 0, "fixture classes: {v}/{h}/{u}");
    assert!(submit_wash(0, 1.0, obs, &p).is_some());
    let mut frames = 0;
    while step_wash(&slow_blocked) == Some(false) {
        frames += 1;
        assert!(frames < 10_000, "wash never finished");
    }
    assert!(
        frames > 1,
        "the wash really was sliced across frames (took {frames})"
    );
    let sliced = take_wash().expect("finished wash");
    assert_eq!(sliced, sync);
    assert!(sliced.cells.contains(&Visibility::Hidden));
}

/// An over-cap submit is REFUSED, leaves the lane idle, and records a message naming the cap and
/// the measured value.
#[test]
fn an_over_cap_submit_is_refused_with_a_message() {
    let over = WashParams {
        radius_m: MAX_WASH_RADIUS_M + 1.0,
        ..WashParams::default()
    };
    assert!(submit_wash(0, 1.0, [0.0, 1.0, 0.0], &over).is_none());
    assert_eq!(active_generation(ViewshedTool::BuildingWash), None);
    let msg = last_refusal().expect("a refusal was recorded");
    assert!(
        msg.contains("building wash radius (m)") && msg.contains("401") && msg.contains("400"),
        "refusal must name the cap and the measured value: {msg}"
    );
}

/// Native has no DEM sampler registered, so the terrain lane declines rather than fabricating a
/// raster — the same `None` the host reads as "draw nothing".
#[test]
fn the_terrain_lane_declines_without_a_sampler() {
    assert!(submit_terrain(1000.0, 1000.0).is_none());
    assert_eq!(active_generation(ViewshedTool::Terrain), None);
}
