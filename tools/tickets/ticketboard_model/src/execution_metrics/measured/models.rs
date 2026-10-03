//! The receipt row, total and sort types.
//!
//! **Role:** `ErrorRow`, `MeasuredRow`, `Grand`, `MetricsModel`, `MetricsState` and the measured sort
//! types.
//! **Position:** part of `crate::execution_metrics::measured`; read by the Metrics tab.
//! **Signals & state:** none; plain data.
//! **Invariants:** measured types are distinct from the estimated ones; an error row is never part
//! of a total.

use super::*;
// ---- receipts ----
// A receipt parses into `ticket_metrics`' `RunRecord` (with its `TokensConsumed`), the shape
// `platform slice-run` writes and `ticket check` validates: `total` is the four-way sum and
// `reasoning` a sibling observation never summed into it.

// ---- scan ----

/// A malformed receipt: named file + VERBATIM reason. Collected, not fatal —
/// and never silently skipped (see module docs for the fail-closed contrast).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorRow {
    /// Path relative to the repo root (matches the checker's error naming).
    pub rel: String,
    /// The verbatim reason the file was excluded.
    pub reason: String,
}

/// One validated receipt, ready to aggregate.
pub(super) struct LoadedRun {
    pub(super) receipt: RunRecord,
    /// `finished − started` whole seconds; `None` = in flight / unfinished.
    pub(super) elapsed: Option<u64>,
    pub(super) started_ns: i128,
    pub(super) finished_ns: Option<i128>,
}

// ---- aggregation (pure) ----

/// One aggregated table row (per ticket or per agent). Display strings are
/// precomputed at load time — the paint path never formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasuredRow {
    /// The ticket id or the agent name.
    pub key: String,
    /// Valid runs.
    pub runs: u64,
    /// The `tokens_consumed.total` sum.
    pub tokens: u64,
    /// Elapsed-seconds sum over runs that HAVE `finished` — only those.
    pub elapsed: u64,
    /// Runs that carry `finished`.
    pub finished_runs: u64,
    /// Runs with no `finished` stamp — shown, never folded into elapsed.
    pub unfinished: u64,
    /// The earliest `started` stamp.
    pub min_started: String,
    /// `None` when no run of this key has finished.
    pub max_finished: Option<String>,
    /// The run count as displayed.
    pub runs_str: String,
    /// The token sum as displayed.
    pub tokens_str: String,
    /// `"—"` while `finished_runs == 0`: an all-in-flight key has UNKNOWN
    /// elapsed, and `"0s"` would fabricate a number.
    pub elapsed_str: String,
    /// The unfinished count as displayed.
    pub unfinished_str: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The measured dashboard's grand totals.
pub struct Grand {
    /// Valid runs.
    pub runs: u64,
    /// The token sum.
    pub tokens: u64,
    /// The elapsed seconds over finished runs.
    pub elapsed: u64,
    /// Runs that carry `finished`.
    pub finished_runs: u64,
    /// Runs with no `finished` stamp.
    pub unfinished: u64,
    /// Distinct tickets.
    pub tickets: usize,
    /// Distinct agents.
    pub agents: usize,
    /// Precomputed headline. With zero VALID runs it says so — never a zeros
    /// row dressed as data.
    pub strip: String,
}

#[derive(Debug, PartialEq, Eq)]
/// The measured dashboard: per-ticket and per-agent rows, error rows and totals.
pub struct MetricsModel {
    /// Sorted by the active sort (tokens desc on load).
    pub per_ticket: Vec<MeasuredRow>,
    /// Sorted by the active sort (tokens desc on load).
    pub per_agent: Vec<MeasuredRow>,
    /// Malformed files, load order — excluded from every sum, listed verbatim.
    pub errors: Vec<ErrorRow>,
    /// The grand totals.
    pub grand: Grand,
}

impl MetricsModel {
    /// Re-sorts both tables by the given selections.
    pub fn apply_sort(&mut self, sorts: SortPair) {
        sort_rows(&mut self.per_ticket, sorts.ticket);
        sort_rows(&mut self.per_agent, sorts.agent);
    }
}

/// Dashboard state. `NoReceipts` is EXPLICIT (directory absent or empty) — the
/// render is [`no_receipts_text()`], never a table of zeros.
#[derive(Debug, PartialEq, Eq)]
pub enum MetricsState {
    /// No receipts: the dashboard shows the explicit empty state.
    NoReceipts,
    /// The aggregated receipts.
    Loaded(MetricsModel),
}

// ---- sorting (column-click) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// The column a measured table sorts by.
pub enum SortKey {
    /// `tokens_consumed.total` sum — the load-time default, descending.
    #[default]
    Tokens,
    /// Valid runs.
    Runs,
    /// Elapsed seconds over finished runs.
    Elapsed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// One measured table's sort: a column and a direction.
pub struct Sort {
    /// The sorted column.
    pub key: SortKey,
    /// True for descending.
    pub desc: bool,
}

impl Sort {
    /// Header-click rule: same column flips direction, a new column starts desc.
    pub fn toggled(self, key: SortKey) -> Sort {
        Sort {
            key,
            desc: if self.key == key { !self.desc } else { true },
        }
    }
}

/// Independent sort selections for the two tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SortPair {
    /// The per-ticket table's sort.
    pub ticket: Sort,
    /// The per-agent table's sort.
    pub agent: Sort,
}

/// Which table a header click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    /// The per-ticket table.
    Ticket,
    /// The per-agent table.
    Agent,
}

impl Default for Sort {
    fn default() -> Self {
        Self {
            key: SortKey::Tokens,
            desc: true,
        }
    }
}
