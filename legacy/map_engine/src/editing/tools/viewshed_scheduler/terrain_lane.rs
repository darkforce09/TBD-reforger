//! Role: the terrain viewshed disc lane — submit, advance, publish.
//! Position: `editing/tools/viewshed_scheduler` in the map engine.
//! Signals & state: the terrain slot, the registered DEM sampler, and the registered viewshed state.
//! Invariants: a submit cancels the tool's predecessor BEFORE any work, so a rapid second placement
//! never pays for the first. A job whose observer is no longer the placed one is dropped rather than
//! published, so a dismissed wash cannot resurrect itself.

use crate::spatial::los::terrain::scheduler::ViewshedJob;
use crate::spatial::los::terrain::viewshed::{Viewshed, ViewshedParams};

use super::host::{host, now_ms};
use super::lanes::{
    TERRAIN, TerrainSlot, VIEWSHED_BUDGET_MS, ViewshedTool, cancel, next_generation, record_refusal,
};
use crate::editing::tools::line_of_sight::host_registry::{
    publish_viewshed_raster, read_registered_sampler, read_registered_viewshed,
};
use crate::editing::tools::line_of_sight::terrain_survey::{
    PROFILE_STEP_M, VIEWSHED_RADIUS_M, everon_manifest,
};
use crate::editing::tools::line_of_sight::terrain_verdict::EYE_HEIGHT_OBSERVER_M;

/// Is `obs` still the observer the session state holds? A dismissal clears the viewshed state and a
/// second placement replaces its observer — so a job whose observer no longer matches is a job
/// nobody wants, and the pump drops it instead of resurrecting a dismissed wash. This is the
/// dismissal half of "cancel on new placement", and it needs no cooperation from the host's Esc
/// handling.
fn observer_still_placed(obs: (f64, f64)) -> bool {
    read_registered_viewshed()
        .observer
        .is_some_and(|(x, y, _)| x == obs.0 && y == obs.1)
}

/// Build the terrain job for an observer at world `(x, y)`, CANCEL whatever the tool was computing,
/// run ONE budgeted batch, and hand back the raster so far. `None` when no DEM sampler is registered
/// — the caller then draws nothing — or when the request is over the cell cap, in which case
/// [`super::last_refusal`] names the cap and the measured value.
///
/// The returned raster is complete where there is no frame loop to finish it; where a host pump
/// runs, it is the first batch and [`pump_terrain_once`] finishes the disc, republishes it, and
/// signals completion.
pub fn submit_terrain(x: f64, y: f64) -> Option<Viewshed> {
    let sampler = read_registered_sampler()?;
    let manifest = everon_manifest();
    let params = ViewshedParams {
        obs_x: x,
        obs_y: y,
        observer_ground_m: sampler(x, y),
        eye_height_m: EYE_HEIGHT_OBSERVER_M,
        radius_m: VIEWSHED_RADIUS_M,
        cell_m: PROFILE_STEP_M,
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
    publish_viewshed_raster(x, y, snapshot.clone());
    if slot.job.done {
        finish_terrain();
    } else {
        TERRAIN.with(|t| *t.borrow_mut() = Some(slot));
        (host().request_pump)();
    }
    Some(snapshot)
}

/// Advance the terrain lane by one budgeted batch. Returns whether a job is still live afterwards —
/// i.e. whether the host should ask for another frame.
pub fn pump_terrain_once() -> bool {
    // TAKEN out of the cell for the duration of the step: the sampler is host code and a re-entrant
    // borrow here would panic the frame.
    let Some(mut slot) = TERRAIN.with(|t| t.borrow_mut().take()) else {
        return false;
    };
    if !observer_still_placed(slot.obs) {
        // Dismissed or superseded — drop the job with the slot, publish nothing.
        return false;
    }
    let elev = {
        let s = slot.sampler.clone();
        move |x: f64, y: f64| s(x, y)
    };
    slot.job.step(&elev, VIEWSHED_BUDGET_MS, &now_ms);
    if slot.job.done {
        let (x, y) = slot.obs;
        publish_viewshed_raster(x, y, slot.job.into_raster());
        finish_terrain();
        return false;
    }
    TERRAIN.with(|t| *t.borrow_mut() = Some(slot));
    true
}

/// The terrain disc is complete and published. The host is told, so a wash drawn over the first
/// batch can be rebuilt over the FINAL raster.
fn finish_terrain() {
    TERRAIN.with(|t| *t.borrow_mut() = None);
    (host().on_terrain_finished)();
}
