//! The concurrency census and the member-account count of a run.
//!
//! - **Role:** counts, for each whole census window of the measured window, the distinct clients
//!   that completed an expected request in it, and counts the accounts that both refreshed
//!   successfully and received an expected JSON answer.
//! - **Position:** called by the report assembly over every client's records.
//! - **Signals & state:** none; pure functions over the records.
//! - **Invariants:**
//!   - Windows start at the measured window's start and are all one census window long; a
//!     trailing partial window is left out, so a short window never lowers the minimum.
//!   - A record counts toward a window when its outcome is expected and it finished inside that
//!     window; a refresh is an expected request like any other.
//!   - `minimum_concurrent_clients` is the smallest window count, and 0 when no whole window fits.

use std::collections::HashSet;
use std::ops::Range;
use std::time::Duration;

use crate::latency_recording::RequestRecord;
use crate::workload_plan::RequestClass;

/// Distinct clients per whole census window, and their minimum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Census {
    /// Distinct clients with an expected answer in each whole window, in order.
    pub windows: Vec<u32>,
    /// The smallest window count; 0 when no whole window fits.
    pub minimum_concurrent_clients: u32,
}

/// Count the clients with an expected answer in each whole `window` of `measured`.
pub fn census(records: &[RequestRecord], measured: &Range<Duration>, window: Duration) -> Census {
    let span = measured.end.saturating_sub(measured.start).as_nanos();
    let whole = usize::try_from(span / window.as_nanos().max(1)).unwrap_or(usize::MAX);
    let mut clients: Vec<HashSet<u32>> = vec![HashSet::new(); whole];
    for record in records.iter().filter(|record| record.outcome.is_expected()) {
        let Some(offset) = record.finished.checked_sub(measured.start) else {
            continue;
        };
        let index = offset.as_nanos() / window.as_nanos().max(1);
        if let Some(seen) = usize::try_from(index)
            .ok()
            .and_then(|index| clients.get_mut(index))
        {
            seen.insert(record.client);
        }
    }
    let windows: Vec<u32> = clients
        .iter()
        .map(|seen| u32::try_from(seen.len()).unwrap_or(u32::MAX))
        .collect();
    Census {
        minimum_concurrent_clients: windows.iter().copied().min().unwrap_or(0),
        windows,
    }
}

/// Accounts with a successful refresh and at least one expected JSON read or write.
pub fn member_accounts(records: &[RequestRecord]) -> u64 {
    let expected = records.iter().filter(|record| record.outcome.is_expected());
    let (refreshed, answered): (Vec<&RequestRecord>, Vec<&RequestRecord>) =
        expected.partition(|record| record.class == RequestClass::Session);
    let refreshed: HashSet<usize> = refreshed.iter().map(|record| record.account).collect();
    let answered: HashSet<usize> = answered.iter().map(|record| record.account).collect();
    refreshed.intersection(&answered).count() as u64
}

#[cfg(test)]
#[path = "tests/concurrency_census_tests.rs"]
mod tests;
