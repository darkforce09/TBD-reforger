//! Role: the host-registered handles the tool reads: the live ray state, the DEM point sampler, and the viewshed state.
//! Position: `editing/tools/line_of_sight` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: a cell that holds nothing reports the empty answer rather than a stale one; reads clone, so no borrow outlives the call.

use std::cell::RefCell;
use std::rc::Rc;

use crate::spatial::los::terrain::viewshed::Viewshed;

use super::capture::{LosState, ViewshedState};

/// A host's DEM point sampler: world `(x, y)` → ground metres, `None` off coverage.
pub type PointSampler = Rc<dyn Fn(f64, f64) -> Option<f64>>;

/// A host-owned handle parked for the tool to read. `None` means no surface is mounted, which every
/// reader below reports as the empty answer — never as a stale one.
///
/// The cells are `pub` because installing into them is the HOST's job: only the host knows when the
/// surface that owns a handle dies, and the identity-guarded unregister that makes a remount safe
/// belongs to the host's lifecycle, not to the geometry.
pub type HostCell<T> = RefCell<Option<Rc<T>>>;

thread_local! {
    /// The live two-click ray capture state.
    pub static LOS_STATE: HostCell<RefCell<LosState>> = const { RefCell::new(None) };

    /// The DEM point sampler, held as a boxed closure so this module needs no compile-time
    /// knowledge of the host's grid handle type.
    pub static LOS_SAMPLER: RefCell<Option<PointSampler>> = const { RefCell::new(None) };

    /// The live viewshed sub-mode state (placed observer + its raster).
    pub static VIEWSHED_STATE: HostCell<RefCell<ViewshedState>> = const { RefCell::new(None) };
}

/// A snapshot clone of the registered ray state (empty when no host has registered one).
#[must_use]
pub fn read_registered_state() -> LosState {
    LOS_STATE.with(|c| {
        c.borrow()
            .as_ref()
            .map(|rc| *rc.borrow())
            .unwrap_or_default()
    })
}

/// A clone of the registered DEM sampler (`None` when no host has registered one). A sampler that
/// closes over a dropped surface's DEM would answer with elevations from a terrain that is no longer
/// open, so the honest `None` is what an unmounted host must produce.
#[must_use]
pub fn read_registered_sampler() -> Option<PointSampler> {
    LOS_SAMPLER.with(|c| c.borrow().clone())
}

/// A snapshot clone of the registered viewshed state (empty when no host has registered one).
#[must_use]
pub fn read_registered_viewshed() -> ViewshedState {
    VIEWSHED_STATE.with(|c| {
        c.borrow()
            .as_ref()
            .map(|rc| rc.borrow().clone())
            .unwrap_or_default()
    })
}

/// Store a computed raster for the observer at world `(x, y)` in the registered viewshed state, so a
/// pan re-projects the same rect instead of recomputing. The click-time ground Z is the host's own
/// `ViewshedState::place` write, not this one. No-op before a host registers.
pub fn publish_viewshed_raster(x: f64, y: f64, vs: Viewshed) {
    VIEWSHED_STATE.with(|c| {
        if let Some(rc) = c.borrow().as_ref() {
            let mut st = rc.borrow_mut();
            st.observer = Some((x, y, None));
            st.raster = Some(vs);
        }
    });
}
