//! T-938.5 — the editor's VIEWSHED JOB SCHEDULER: one active job per tool, advanced in
//! animation-frame batches of at most [`VIEWSHED_BUDGET_MS`], cancelled by the next placement.
//!
//! # The defect this exists for
//!
//! Placing a viewshed used to run the WHOLE sweep inside the pointer-up handler. Measured on this
//! repo's own envelope tests (debug build, host): the shipped 2000 m / 8 m terrain disc is
//! **56.8 ms** of blocked wasm thread (251,001 cells), and the building wash at its shipped 25 m
//! default is **56.5 ms** for two levels — three-plus dropped frames each, and every one of them
//! inside the click. Radii were unbounded on top of that: a 1000 m building wash measured
//! **5,387 ms**, accepted without complaint.
//!
//! # The shape (copied, not invented)
//!
//! This is `los_world::ObjectPass`'s idiom applied to the two viewshed rasters: the pure, resumable
//! job lives in `map-engine-core` ([`ViewshedJob`], [`WashJob`]) with a cursor, a `generation`
//! stamp and `step(…, budget_ms, now)`; the core holds NO cancellation state — it returns after its
//! budget and this module decides whether to call again. The caps are the core's too
//! ([`ViewshedJob::new`] / [`WashJob::new`] return [`ViewshedCapRefused`]); over-cap requests are
//! refused here with the message the core supplies, readable via [`last_refusal`].
//!
//! # Two lanes, deliberately asymmetric
//!
//! * **Terrain** ([`submit_terrain`]) is pumped by this module's OWN `requestAnimationFrame`
//!   closure. Its work needs only the registered DEM sampler (an `Rc<dyn Fn>` that outlives a
//!   frame), so the job can be parked in a thread-local and driven without its submitter.
//! * **Building wash** ([`submit_wash`] / [`step_wash`]) is stepped BY ITS OWNER. A wash needs a
//!   `blocked` closure that borrows the building's BVH / compound for the call, which cannot be
//!   parked in a `'static` thread-local; the scheduler still owns the SLOT — so "one active job per
//!   tool" holds and a new placement cancels the old — and the owner supplies the closure per step.
//!
//! The rAF closure is this module's own on purpose: `canvas/viewport.rs` (which pumps the object
//! wash) and `canvas/gestures.rs` (which calls `place_viewshed` and uploads) both stay untouched, so
//! `place_viewshed`'s call-site contract — and the `t644_viewshed_wiring` source pin requiring
//! `place_viewshed(` to precede `viewshed_upload(` — are unchanged.

// The same gate `canvas/viewport.rs` carries, for a narrower reason. On the wasm build the test
// module is not compiled, and the WASH lane's production consumer — the multi-floor viewer in
// `pages/debug/building_viewer.rs` — is owned by no one this pack, so nothing on the wasm side calls
// `submit_wash` / `step_wash` / `take_wash` yet; the readouts (`last_refusal`, `progress`,
// `active_generation`) are the operator-facing surface a HUD cell will read. All of them ARE
// exercised, natively, by this module's own tests. Deleting them to silence the lint would drop the
// half of "one active job per TOOL" that makes the phrase mean anything.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use map_engine_core::building_viewshed::{LevelWash, WashJob, WashParams};
use map_engine_core::dem::sample::{Viewshed, ViewshedCapRefused, ViewshedJob, ViewshedParams};

use super::los_tool;

/// Per-frame compute budget for ONE viewshed batch, milliseconds. Well inside a 16.7 ms frame with
/// the renderer's own work (and the object wash's `OBJECT_PASS_BUDGET_MS`) still to pay for.
pub const VIEWSHED_BUDGET_MS: f64 = 4.0;

/// Which tool a job belongs to. The slot is keyed by this, so at most ONE job per tool is live and
/// a submit cancels only its own tool's predecessor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewshedTool {
    /// The LoS tool's terrain viewshed disc (`los_tool::place_viewshed`).
    Terrain,
    /// A building's per-level visibility wash (the multi-floor viewer).
    BuildingWash,
}

/// The terrain lane's live job plus what the pump needs to finish it without its submitter: the
/// observer it was placed for (the dismissal check) and the DEM sampler it marches over.
struct TerrainSlot {
    job: ViewshedJob,
    obs: (f64, f64),
    sampler: Rc<dyn Fn(f64, f64) -> Option<f64>>,
}

thread_local! {
    static TERRAIN: RefCell<Option<TerrainSlot>> = const { RefCell::new(None) };
    static WASH: RefCell<Option<WashJob>> = const { RefCell::new(None) };
    /// Monotonic cancel token — bumped on EVERY submit, so a job's stamp identifies its placement.
    static GENERATION: Cell<u32> = const { Cell::new(0) };
    /// The last cap refusal, for the operator-facing readout.
    static LAST_REFUSAL: RefCell<Option<String>> = const { RefCell::new(None) };
    /// One rAF closure at a time, however many placements arrive.
    #[cfg(target_arch = "wasm32")]
    static PUMPING: Cell<bool> = const { Cell::new(false) };
}

/// Milliseconds from an arbitrary epoch — the clock the budget is measured against.
fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
    }
}

fn next_generation() -> u32 {
    GENERATION.with(|g| {
        g.set(g.get().wrapping_add(1));
        g.get()
    })
}

/// Record a cap refusal so the operator can see WHY nothing was drawn, and log it once.
fn record_refusal(refused: &ViewshedCapRefused) {
    let msg = refused.to_string();
    leptos::logging::warn!("{}", msg);
    LAST_REFUSAL.with(|r| *r.borrow_mut() = Some(msg));
}

/// The most recent cap refusal message — which cap, and the measured value that broke it. `None`
/// until a request is refused; it is NOT cleared by a later success (it is a diagnostic, and the
/// operator may read it after the fact).
#[must_use]
pub fn last_refusal() -> Option<String> {
    LAST_REFUSAL.with(|r| r.borrow().clone())
}

/// Drop `tool`'s live job, if any. Returns whether one was dropped.
pub fn cancel(tool: ViewshedTool) -> bool {
    match tool {
        ViewshedTool::Terrain => TERRAIN.with(|t| t.borrow_mut().take().is_some()),
        ViewshedTool::BuildingWash => WASH.with(|w| w.borrow_mut().take().is_some()),
    }
}

/// The generation stamp of `tool`'s live job, or `None` when the lane is idle.
#[must_use]
pub fn active_generation(tool: ViewshedTool) -> Option<u32> {
    match tool {
        ViewshedTool::Terrain => TERRAIN.with(|t| t.borrow().as_ref().map(|s| s.job.generation)),
        ViewshedTool::BuildingWash => WASH.with(|w| w.borrow().as_ref().map(|j| j.generation)),
    }
}

/// `(done, total)` progress for `tool`'s live job — rays for the terrain lane, cells for the wash.
#[must_use]
pub fn progress(tool: ViewshedTool) -> Option<(usize, usize)> {
    match tool {
        ViewshedTool::Terrain => TERRAIN.with(|t| t.borrow().as_ref().map(|s| s.job.progress())),
        ViewshedTool::BuildingWash => WASH.with(|w| w.borrow().as_ref().map(WashJob::progress)),
    }
}

// ── The terrain lane ────────────────────────────────────────────────────────────────────────────

/// Is `obs` still the observer the session state holds? Esc (`canvas/commands.rs`) clears the
/// session `ViewshedState`, and a second placement replaces its observer — so a job whose observer
/// no longer matches is a job nobody wants, and the pump drops it instead of resurrecting a
/// dismissed wash. This is the dismissal half of "cancel on new placement", and it needs no edit to
/// the (sibling-owned) Esc handler.
fn observer_still_placed(obs: (f64, f64)) -> bool {
    los_tool::read_registered_viewshed()
        .observer
        .is_some_and(|(x, y, _)| x == obs.0 && y == obs.1)
}

/// The T-938.5 submission `los_tool::place_viewshed` makes: build the terrain job for an observer at
/// world `(x, y)`, CANCEL whatever the tool was computing, run ONE budgeted batch, and hand back the
/// raster so far. `None` when no DEM sampler is registered (native / pre-mount — the caller then
/// draws nothing) or when the request is over the cell cap, in which case [`last_refusal`] names the
/// cap and the measured value.
///
/// The returned raster is complete on native (there is no frame loop to finish it, and a native
/// caller expects what the synchronous path returned); on wasm it is the first batch, and the rAF
/// pump finishes the disc, republishes it and restarts the object wash over the FINAL raster.
pub fn submit_terrain(x: f64, y: f64) -> Option<Viewshed> {
    let sampler = los_tool::read_registered_sampler()?;
    let manifest = los_tool::everon_manifest();
    let params = ViewshedParams {
        obs_x: x,
        obs_y: y,
        observer_ground_m: sampler(x, y),
        eye_height_m: los_tool::EYE_HEIGHT_OBSERVER_M,
        radius_m: los_tool::VIEWSHED_RADIUS_M,
        cell_m: los_tool::PROFILE_STEP_M,
    };
    let generation = next_generation();
    // CANCEL ON NEW PLACEMENT — before any work, so a rapid second click never pays for the first.
    cancel(ViewshedTool::Terrain);
    let job = match ViewshedJob::new(&manifest, params, generation) {
        Ok(job) => job,
        Err(refused) => {
            record_refusal(&refused);
            return None;
        }
    };
    let mut slot = TerrainSlot {
        job,
        obs: (x, y),
        sampler,
    };
    let elev = {
        let s = slot.sampler.clone();
        move |x: f64, y: f64| s(x, y)
    };
    slot.job.step(&elev, VIEWSHED_BUDGET_MS, &now_ms);
    #[cfg(not(target_arch = "wasm32"))]
    while !slot.job.done {
        slot.job.step(&elev, VIEWSHED_BUDGET_MS, &now_ms);
    }
    let snapshot = slot.job.raster().clone();
    los_tool::publish_viewshed_raster(x, y, snapshot.clone());
    if slot.job.done {
        finish_terrain();
    } else {
        TERRAIN.with(|t| *t.borrow_mut() = Some(slot));
        start_pump();
    }
    Some(snapshot)
}

/// Advance the terrain lane by one budgeted batch. Returns whether a job is still live afterwards
/// (i.e. whether the pump should ask for another frame).
fn pump_terrain_once() -> bool {
    // TAKEN out of the cell for the duration of the step: the sampler is host code and a re-entrant
    // borrow here would panic the frame (the `canvas/viewport.rs` T-631 lesson).
    let Some(mut slot) = TERRAIN.with(|t| t.borrow_mut().take()) else {
        return false;
    };
    if !observer_still_placed(slot.obs) {
        // Dismissed (Esc) or superseded — drop the job with the slot, publish nothing.
        return false;
    }
    let elev = {
        let s = slot.sampler.clone();
        move |x: f64, y: f64| s(x, y)
    };
    slot.job.step(&elev, VIEWSHED_BUDGET_MS, &now_ms);
    if slot.job.done {
        let (x, y) = slot.obs;
        los_tool::publish_viewshed_raster(x, y, slot.job.into_raster());
        finish_terrain();
        return false;
    }
    TERRAIN.with(|t| *t.borrow_mut() = Some(slot));
    true
}

/// The terrain disc is complete and published. Restart the object wash so its merged upload is over
/// the FINAL raster — `canvas/gestures.rs` starts one right after `place_viewshed`, which (now that
/// the disc is sliced) began over the first batch. Re-starting retires that generation and rebuilds
/// the pass over the finished terrain; the rAF `tick_object_wash` then uploads the merge.
fn finish_terrain() {
    TERRAIN.with(|t| *t.borrow_mut() = None);
    #[cfg(target_arch = "wasm32")]
    super::los_world_wasm::start_object_wash();
}

/// This module's OWN self-rescheduling `requestAnimationFrame` closure (the `canvas/viewport.rs`
/// shape), started on submit and dropped as soon as no job is live. One at a time, however many
/// placements arrive.
#[cfg(target_arch = "wasm32")]
type PumpClosure = Rc<RefCell<Option<wasm_bindgen::prelude::Closure<dyn FnMut()>>>>;

#[cfg(target_arch = "wasm32")]
fn start_pump() {
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;

    if PUMPING.with(Cell::get) {
        return;
    }
    PUMPING.with(|p| p.set(true));
    let f: PumpClosure = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if !pump_terrain_once() {
            PUMPING.with(|p| p.set(false));
            f.borrow_mut().take(); // drop the loop closure — no further frames
            return;
        }
        let cb_ref = f.borrow();
        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut()>));
    let cb_ref = g.borrow();
    if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    } else {
        PUMPING.with(|p| p.set(false));
    }
}

/// Native builds have no frame loop — [`submit_terrain`] already drained the job, so there is
/// nothing to pump. (Kept as a peer of the wasm arm so the submit path reads the same on both.)
#[cfg(not(target_arch = "wasm32"))]
fn start_pump() {}

// ── The building-wash lane ──────────────────────────────────────────────────────────────────────

/// Submit a building wash for `(level_index, eye_y, obs)` under `p`, cancelling whatever the wash
/// tool was computing. Returns the job's generation stamp, or `None` when the radius is over the
/// cap — [`last_refusal`] then names the cap and the measured radius.
///
/// The wash is stepped by its OWNER (see the module note): call [`step_wash`] each frame with the
/// `blocked` closure in hand, then [`take_wash`] when it reports done.
pub fn submit_wash(level_index: usize, eye_y: f64, obs: [f64; 3], p: &WashParams) -> Option<u32> {
    let generation = next_generation();
    cancel(ViewshedTool::BuildingWash);
    match WashJob::new(level_index, eye_y, obs, p, generation) {
        Ok(job) => {
            WASH.with(|w| *w.borrow_mut() = Some(job));
            Some(generation)
        }
        Err(refused) => {
            record_refusal(&refused);
            None
        }
    }
}

/// Advance the wash lane by one budgeted batch under `blocked`. `Some(done)` reports whether the
/// wash finished; `None` when no wash job is live.
pub fn step_wash(blocked: &dyn Fn([f64; 3], [f64; 3]) -> bool) -> Option<bool> {
    WASH.with(|w| {
        let mut guard = w.borrow_mut();
        let job = guard.as_mut()?;
        job.step(blocked, VIEWSHED_BUDGET_MS, &now_ms);
        Some(job.done)
    })
}

/// Take the wash raster out of the lane (finished or partial), leaving it idle.
#[must_use]
pub fn take_wash() -> Option<LevelWash> {
    WASH.with(|w| w.borrow_mut().take().map(WashJob::into_wash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_engine_core::building_viewshed::{wash_band, MAX_WASH_RADIUS_M, WASH_BATCH_CELLS};
    use map_engine_core::dem::sample::Visibility;

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
}
