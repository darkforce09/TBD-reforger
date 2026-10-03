//! The workload's numbers as a run applies them: checked once and converted to durations and
//! ceiling windows.
//!
//! - **Role:** declares [`RunSettings`] and [`CeilingWindow`], which
//!   [`crate::workload_plan::WorkloadPlan::checked_settings`] builds, and the two conversions it
//!   builds them with: seconds to a duration and a per-address ceiling to a window.
//! - **Position:** built by the workload and run checks; read by the pacing, the report assembly
//!   and the load generator's clients and address guard.
//! - **Signals & state:** none; plain values and pure conversions.
//! - **Invariants:** every duration is finite and above zero (the ramp may be zero), and every
//!   ceiling allows at least one request per window.

use std::time::Duration;

use crate::error::{Error, Result, refuse_unless};
use crate::workload_plan::WindowCeiling;

/// A ceiling as the guard applies it.
#[derive(Debug, Clone, Copy)]
pub struct CeilingWindow {
    /// Request starts allowed inside one window.
    pub max_requests: usize,
    /// Length of the sliding window.
    pub window: Duration,
}

/// The workload's numbers, checked and converted once.
#[derive(Debug, Clone, Copy)]
pub struct RunSettings {
    /// Virtual clients.
    pub clients: u32,
    /// Accounts in the account file: `clients × accounts_per_client`.
    pub accounts: usize,
    /// The paced period of one client: `clients / requests_per_second`.
    pub period: Duration,
    /// The sign-in ramp; nothing in it is measured.
    pub ramp: Duration,
    /// The measured window after the ramp.
    pub measured: Duration,
    /// How long a client stays on one account.
    pub hold: Duration,
    /// Jitter of every paced slot as a fraction of the period, both ways.
    pub jitter_fraction: f64,
    /// The total timeout of one request.
    pub request_timeout: Duration,
    /// Width of the concurrency census windows.
    pub census_window: Duration,
    /// The ceiling on every request from one source address.
    pub all_requests: CeilingWindow,
    /// The ceiling on the authentication requests from one source address.
    pub auth_requests: CeilingWindow,
}

/// `value` seconds as a duration, refused unless finite and above zero (or zero when allowed).
pub(crate) fn seconds(name: &str, value: f64, allow_zero: bool) -> Result<Duration> {
    let in_range = if allow_zero {
        value >= 0.0
    } else {
        value > 0.0
    };
    refuse_unless!(
        value.is_finite() && in_range,
        "{name} must be a finite number of seconds {} (got {value})",
        if allow_zero {
            "of at least 0"
        } else {
            "above 0"
        }
    );
    Duration::try_from_secs_f64(value)
        .map_err(|error| Error::refused(format!("{name} = {value}: {error}")))
}

pub(crate) fn checked_ceiling(name: &str, ceiling: WindowCeiling) -> Result<CeilingWindow> {
    refuse_unless!(
        ceiling.max_requests >= 1,
        "per_address_ceilings.{name}.max_requests must be at least 1"
    );
    let window = seconds(
        &format!("per_address_ceilings.{name}.window_seconds"),
        ceiling.window_seconds,
        false,
    )?;
    Ok(CeilingWindow {
        max_requests: ceiling.max_requests as usize,
        window,
    })
}
