//! Role: the building-level visibility wash lane.
//! Position: `editing/tools/viewshed_scheduler` in the map engine.
//! Signals & state: the wash slot.
//! Invariants: the scheduler owns the slot — so one active job per tool holds and a new submission
//! cancels the old — while the OWNER supplies the `blocked` closure per step, because that closure
//! borrows a building's geometry for the call and cannot be parked in a `'static` slot.

use crate::spatial::los::interior::wash::{LevelWash, WashJob, WashParams};

use super::host::now_ms;
use super::lanes::{
    VIEWSHED_BUDGET_MS, ViewshedTool, WASH, cancel, next_generation, record_refusal,
};

/// Submit a building wash for `(level_index, eye_y, obs)` under `p`, cancelling whatever the wash
/// tool was computing. Returns the job's generation stamp, or `None` when the radius is over the
/// cap — [`super::last_refusal`] then names the cap and the measured radius.
///
/// The wash is stepped by its OWNER: call [`step_wash`] each frame with the `blocked` closure in
/// hand, then [`take_wash`] when it reports done.
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
