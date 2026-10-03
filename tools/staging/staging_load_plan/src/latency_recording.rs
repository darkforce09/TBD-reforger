//! What each exchange did, and the nearest-rank percentiles of a class's latencies.
//!
//! - **Role:** defines the [`RequestRecord`] every exchange leaves behind and its
//!   [`RequestOutcome`], and summarises a class's latencies inside a window by nearest rank.
//! - **Position:** the virtual clients write records; the census and the report read them.
//! - **Signals & state:** none; plain data and pure functions.
//! - **Invariants:**
//!   - Latency is `finished − scheduled`: from the event's scheduled instant to the end of the
//!     answer's body, so a client that fell behind carries its delay into the sample.
//!   - An outcome is expected only when a response arrived with a status its step lists; any other
//!     status, a transport error, a timeout or an undecodable refresh answer is unexpected.
//!   - The `p`-th percentile is the value at 1-based rank `⌈p·n / 100⌉` of the ascending sample,
//!     computed in integers; an empty sample has none.

use std::ops::Range;
use std::time::Duration;

use crate::workload_plan::RequestClass;

/// How one exchange ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// A response whose status the step lists.
    Expected {
        /// The response's status.
        status: u16,
    },
    /// A response whose status the step does not list.
    UnexpectedStatus {
        /// The response's status.
        status: u16,
    },
    /// No complete response: refused, reset or cut off before the body ended.
    TransportError,
    /// The request timeout passed before the body ended.
    Timeout,
    /// A refresh answered with an expected status but not with a complete token pair.
    UndecodableSessionAnswer {
        /// The response's status.
        status: u16,
    },
}

impl RequestOutcome {
    /// The outcome of a response with `status` for a step that expects `expected`.
    pub fn classify(status: u16, expected: &[u16]) -> Self {
        if expected.contains(&status) {
            Self::Expected { status }
        } else {
            Self::UnexpectedStatus { status }
        }
    }

    /// Whether a response arrived with a status its step lists.
    pub fn is_expected(self) -> bool {
        matches!(self, Self::Expected { .. })
    }

    /// The status of the response, when one arrived whole.
    pub fn status(self) -> Option<u16> {
        match self {
            Self::Expected { status }
            | Self::UnexpectedStatus { status }
            | Self::UndecodableSessionAnswer { status } => Some(status),
            Self::TransportError | Self::Timeout => None,
        }
    }
}

/// One exchange; instants are offsets from the run start.
#[derive(Debug, Clone, Copy)]
pub struct RequestRecord {
    /// The client that sent it.
    pub client: u32,
    /// The index of its source address in the plan.
    pub address: usize,
    /// The index of the account it was sent as.
    pub account: usize,
    /// The latency sample it joins.
    pub class: RequestClass,
    /// The mix template, or `None` for a refresh.
    pub template: Option<usize>,
    /// The instant its slot or switch was scheduled for.
    pub scheduled: Duration,
    /// The instant it left, after the address's guard.
    pub sent: Duration,
    /// When the body ended or the failure surfaced.
    pub finished: Duration,
    /// How it ended.
    pub outcome: RequestOutcome,
    /// The request passed the per-address auth ceiling.
    pub auth: bool,
}

impl RequestRecord {
    /// `finished − scheduled`: the wait behind its schedule counts as latency.
    pub fn latency(&self) -> Duration {
        self.finished.saturating_sub(self.scheduled)
    }
}

/// The latency sample of one class inside one window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LatencySummary {
    /// Expected answers in the sample.
    pub samples: usize,
    /// The nearest-rank median.
    pub p50: Option<Duration>,
    /// The nearest-rank 95th percentile.
    pub p95: Option<Duration>,
    /// The slowest answer.
    pub max: Option<Duration>,
}

/// The value at 1-based rank `⌈percent·n / 100⌉` of `sorted`, which must be ascending.
pub fn nearest_rank(sorted: &[Duration], percent: usize) -> Option<Duration> {
    let rank = (percent * sorted.len()).div_ceil(100).max(1);
    sorted.get(rank - 1).copied()
}

/// Summarise the expected answers of `class` that finished inside `window`.
pub fn summarise(
    records: &[RequestRecord],
    class: RequestClass,
    window: &Range<Duration>,
) -> LatencySummary {
    let mut latencies: Vec<Duration> = records
        .iter()
        .filter(|record| {
            record.class == class
                && record.outcome.is_expected()
                && window.contains(&record.finished)
        })
        .map(RequestRecord::latency)
        .collect();
    latencies.sort_unstable();
    LatencySummary {
        samples: latencies.len(),
        p50: nearest_rank(&latencies, 50),
        p95: nearest_rank(&latencies, 95),
        max: latencies.last().copied(),
    }
}

#[cfg(test)]
#[path = "tests/latency_recording_tests.rs"]
mod tests;
