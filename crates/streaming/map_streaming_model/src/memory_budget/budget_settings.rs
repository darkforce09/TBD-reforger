//! The budget a boot's settings resolve to.
//!
//! **Role:** [`budget_bytes_from_settings`]: the `memBudgetMb` query parameter, else the page's
//! `window.__memBudgetMb` global, else [`DEFAULT_BUDGET_MB`], in bytes.
//! **Position:** called by the map engine's live ledger with the values it read from the page,
//! and with neither on a native build.
//! **Signals & state:** none; a pure function.
//! **Invariants:** a setting counts only when it is a positive number of MiB; the result is
//! that number times [`MIB`], saturating.

use super::model::{DEFAULT_BUDGET_MB, MIB};

/// The budget in bytes for a boot's settings.
///
/// `query_budget_mb` is the raw `memBudgetMb` query value, read when it parses as a whole number
/// of MiB after trimming; `window_budget_mb` is the `window.__memBudgetMb` global, read when the
/// query gave none and it is positive and finite. A zero or missing setting falls back to
/// [`DEFAULT_BUDGET_MB`].
#[must_use]
pub fn budget_bytes_from_settings(
    query_budget_mb: Option<&str>,
    window_budget_mb: Option<f64>,
) -> u64 {
    let from_query = query_budget_mb.and_then(|value| value.trim().parse::<u64>().ok());
    let from_window = || {
        window_budget_mb
            .filter(|mb| *mb > 0.0 && mb.is_finite())
            .map(|mb| mb as u64)
    };
    from_query
        .or_else(from_window)
        .filter(|mb| *mb > 0)
        .unwrap_or(DEFAULT_BUDGET_MB)
        .saturating_mul(MIB)
}
