//! Role: the host services the scheduler cannot supply itself.
//! Position: `viewshed_scheduler` in `map_editing_tools`; the Mission Creator installs a table at
//! canvas mount, and both lanes read it.
//! Signals & state: one installed service table per thread.
//! Invariants: every service has a defined default, so an uninstalled scheduler neither stalls a
//! budget loop nor panics. The default clock is [`time_source::monotonic_ms`], which never runs
//! backwards on any target, so a budget loop terminates with no host present; refusals, pump
//! requests and the completion signal default to doing nothing. Without a pump, a native build
//! finishes a terrain disc inside [`crate::viewshed_scheduler::submit_terrain`], while a `wasm32`
//! build returns its first batch and parks the rest until a host pump calls
//! [`crate::viewshed_scheduler::pump_terrain_once`].

use std::cell::Cell;

/// What the scheduler asks of whoever is driving it.
///
/// Plain function pointers, not closures: the table is `Copy`, holds no captured state, and can be
/// installed once at boot. A host that supplies none of them still gets correct results, silently;
/// see the module header for what an absent pump means on each target.
#[derive(Clone, Copy)]
pub struct SchedulerHost {
    /// Milliseconds from an arbitrary epoch — the clock every budget is measured against.
    pub now_ms: fn() -> f64,
    /// Report a cap refusal to the operator. The message names the cap and the measured value.
    pub report_refusal: fn(&str),
    /// A terrain job is live and wants frames. The host starts (or keeps) its frame pump, which
    /// calls [`crate::viewshed_scheduler::pump_terrain_once`] until it answers `false`.
    pub request_pump: fn(),
    /// The terrain disc is complete and published.
    pub on_terrain_finished: fn(),
}

fn no_report(_: &str) {}
fn no_pump() {}
fn no_finish() {}

impl Default for SchedulerHost {
    fn default() -> Self {
        Self {
            now_ms: time_source::monotonic_ms,
            report_refusal: no_report,
            request_pump: no_pump,
            on_terrain_finished: no_finish,
        }
    }
}

thread_local! {
    static HOST: Cell<SchedulerHost> = const {
        Cell::new(SchedulerHost {
            now_ms: time_source::monotonic_ms,
            report_refusal: no_report,
            request_pump: no_pump,
            on_terrain_finished: no_finish,
        })
    };
}

/// Install the host service table. Later installs replace earlier ones wholesale.
pub fn install_host(host: SchedulerHost) {
    HOST.with(|h| h.set(host));
}

/// The installed services.
#[must_use]
pub fn host() -> SchedulerHost {
    HOST.with(Cell::get)
}

/// The clock every budget is measured against.
#[must_use]
pub fn now_ms() -> f64 {
    (host().now_ms)()
}
