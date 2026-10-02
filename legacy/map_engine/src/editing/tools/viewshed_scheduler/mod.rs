//! Role: Module boundary for the viewshed job scheduler.
//! Position: `editing/tools` in the map engine.
//! Signals & state: one live job per tool, a monotonic cancel token, and the last cap refusal.
//! Invariants: at most ONE job per tool is live, and a submit cancels only its own tool's
//! predecessor. Every batch is bounded by [`VIEWSHED_BUDGET_MS`] of the host clock, so no single
//! step can run away with a frame. An over-cap request is refused with the cap it broke rather
//! than accepted and run.
//!
//! # Two lanes, deliberately asymmetric
//!
//! * **Terrain** ([`submit_terrain`]) is advanced by [`pump_terrain_once`], which the host calls
//!   once a frame. Its work needs only the registered DEM sampler — an `Rc<dyn Fn>` that outlives
//!   a frame — so the job can be parked here and driven without its submitter.
//! * **Building wash** ([`submit_wash`] / [`step_wash`]) is stepped BY ITS OWNER. A wash needs a
//!   `blocked` closure that borrows the building's BVH for the call, which cannot be parked in a
//!   `'static` slot; the scheduler still owns the SLOT — so "one active job per tool" holds and a
//!   new placement cancels the old — and the owner supplies the closure per step.

/// The host services the scheduler cannot supply itself: a clock, a refusal sink, a frame pump,
/// and the completion signal.
pub mod host;

/// The building-level visibility wash lane.
pub mod wash_lane;

/// The terrain disc lane.
pub mod terrain_lane;

/// The slots, the cancel token, and the readouts both lanes share.
pub mod lanes;

pub use host::{SchedulerHost, install_host};
pub use lanes::{
    VIEWSHED_BUDGET_MS, ViewshedTool, active_generation, cancel, last_refusal, progress,
};
pub use terrain_lane::{pump_terrain_once, submit_terrain};
pub use wash_lane::{step_wash, submit_wash, take_wash};

#[cfg(test)]
#[path = "tests/lane_isolation.rs"]
mod tests;
