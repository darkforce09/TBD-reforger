//! Role: the slots both lanes live in, the cancel token, and the readouts over them.
//! Position: `editing/tools/viewshed_scheduler` in the map engine.
//! Signals & state: one slot per tool, a monotonic generation, and the last cap refusal.
//! Invariants: the slot is keyed by tool, so at most ONE job per tool is live and a submit cancels
//! only its own tool's predecessor. The generation is monotonic, so a job's stamp identifies its
//! placement. A refusal is a diagnostic and is never cleared by a later success.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::spatial::los::interior::wash::WashJob;
use crate::spatial::los::terrain::scheduler::ViewshedJob;
use crate::spatial::los::terrain::viewshed::ViewshedCapRefused;

/// Per-frame compute budget for ONE batch, milliseconds. Well inside a 16.7 ms frame with a
/// renderer's own work, and the object wash's own budget, still to pay for.
pub const VIEWSHED_BUDGET_MS: f64 = 4.0;

/// Which tool a job belongs to. The slot is keyed by this, so at most ONE job per tool is live and
/// a submit cancels only its own tool's predecessor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewshedTool {
    /// The line-of-sight tool's terrain viewshed disc.
    Terrain,
    /// A building's per-level visibility wash.
    BuildingWash,
}

/// The terrain lane's live job plus what a pump needs to finish it without its submitter: the
/// observer it was placed for (the dismissal check) and the DEM sampler it marches over.
pub(super) struct TerrainSlot {
    pub(super) job: ViewshedJob,
    pub(super) obs: (f64, f64),
    pub(super) sampler: Rc<dyn Fn(f64, f64) -> Option<f64>>,
}

thread_local! {
    pub(super) static TERRAIN: RefCell<Option<TerrainSlot>> = const { RefCell::new(None) };
    pub(super) static WASH: RefCell<Option<WashJob>> = const { RefCell::new(None) };
    /// Monotonic cancel token — bumped on EVERY submit, so a job's stamp identifies its placement.
    static GENERATION: Cell<u32> = const { Cell::new(0) };
    /// The last cap refusal, for the operator-facing readout.
    static LAST_REFUSAL: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(super) fn next_generation() -> u32 {
    GENERATION.with(|g| {
        g.set(g.get().wrapping_add(1));
        g.get()
    })
}

/// Record a cap refusal so the operator can see WHY nothing was drawn, and report it once.
pub(super) fn record_refusal(refused: &ViewshedCapRefused) {
    let msg = refused.to_string();
    (super::host::host().report_refusal)(&msg);
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
