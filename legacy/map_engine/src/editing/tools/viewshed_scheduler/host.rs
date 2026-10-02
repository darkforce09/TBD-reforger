//! Role: the host services the scheduler cannot supply itself.
//! Position: `editing/tools/viewshed_scheduler` in the map engine.
//! Signals & state: one installed service table per thread.
//! Invariants: every service has a defined default, so an uninstalled scheduler degrades to
//! "finish the work here and now" rather than stalling or panicking. The fallback clock always
//! advances, so a budget loop terminates even with no host present.

use std::cell::Cell;

/// What the scheduler asks of whoever is driving it.
///
/// Plain function pointers, not closures: the table is `Copy`, holds no captured state, and can be
/// installed once at boot. A host that supplies none of them still gets correct results — just
/// synchronously, and silently.
#[derive(Clone, Copy)]
pub struct SchedulerHost {
    /// Milliseconds from an arbitrary epoch — the clock every budget is measured against.
    pub now_ms: fn() -> f64,
    /// Report a cap refusal to the operator. The message names the cap and the measured value.
    pub report_refusal: fn(&str),
    /// A terrain job is live and wants frames. The host starts (or keeps) its frame pump, which
    /// calls [`super::pump_terrain_once`] until it answers `false`.
    pub request_pump: fn(),
    /// The terrain disc is complete and published.
    pub on_terrain_finished: fn(),
}

fn fallback_now_ms() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
    }
    // No wall clock is reachable without a host. The counter still ADVANCES, which is the only
    // property a budget loop needs to terminate: a step decides one block and hands the frame back.
    #[cfg(target_arch = "wasm32")]
    {
        thread_local! {
            static TICKS: Cell<f64> = const { Cell::new(0.0) };
        }
        TICKS.with(|t| {
            let v = t.get() + 1.0;
            t.set(v);
            v
        })
    }
}

fn no_report(_: &str) {}
fn no_pump() {}
fn no_finish() {}

impl Default for SchedulerHost {
    fn default() -> Self {
        Self {
            now_ms: fallback_now_ms,
            report_refusal: no_report,
            request_pump: no_pump,
            on_terrain_finished: no_finish,
        }
    }
}

thread_local! {
    static HOST: Cell<SchedulerHost> = const {
        Cell::new(SchedulerHost {
            now_ms: fallback_now_ms,
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
