use super::*;
// ---- receipt mirror (read-only) ----

/// Mirror of xtask `TokensConsumed`. `total` is ALWAYS the four-way sum;
/// `reasoning` is a sibling observation and is NEVER summed into `total`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TokensConsumed {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total: u64,
    #[serde(default)]
    pub reasoning: Option<u64>,
}

/// Mirror of xtask `RunRecord`. Field optionality mirrors the committed schema:
/// `id` / `agent` / `started` / `tokens_consumed` required; `finished`,
/// `outcome`, `git_sha` are stamps `platform wave land` writes later.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunReceipt {
    pub id: String,
    pub agent: String,
    pub started: String,
    #[serde(default)]
    pub finished: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub git_sha: Option<String>,
    pub tokens_consumed: TokensConsumed,
}

// ---- scan ----

/// A malformed receipt: named file + VERBATIM reason. Collected, not fatal —
/// and never silently skipped (see module docs for the fail-closed contrast).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorRow {
    /// Path relative to the repo root (matches the checker's error naming).
    pub rel: String,
    pub reason: String,
}

/// One validated receipt, ready to aggregate.
pub(super) struct LoadedRun {
    pub(super) receipt: RunReceipt,
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
    pub key: String,
    pub runs: u64,
    pub tokens: u64,
    /// Elapsed-seconds sum over runs that HAVE `finished` — only those.
    pub elapsed: u64,
    pub finished_runs: u64,
    /// Runs with no `finished` stamp — shown, never folded into elapsed.
    pub unfinished: u64,
    pub min_started: String,
    /// `None` when no run of this key has finished.
    pub max_finished: Option<String>,
    pub runs_str: String,
    pub tokens_str: String,
    /// `"—"` while `finished_runs == 0`: an all-in-flight key has UNKNOWN
    /// elapsed, and `"0s"` would fabricate a number.
    pub elapsed_str: String,
    pub unfinished_str: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grand {
    pub runs: u64,
    pub tokens: u64,
    pub elapsed: u64,
    pub finished_runs: u64,
    pub unfinished: u64,
    pub tickets: usize,
    pub agents: usize,
    /// Precomputed headline. With zero VALID runs it says so — never a zeros
    /// row dressed as data.
    pub strip: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MetricsModel {
    /// Sorted by the active sort (tokens desc on load).
    pub per_ticket: Vec<MeasuredRow>,
    pub per_agent: Vec<MeasuredRow>,
    /// Malformed files, load order — excluded from every sum, listed verbatim.
    pub errors: Vec<ErrorRow>,
    pub grand: Grand,
}

impl MetricsModel {
    pub fn apply_sort(&mut self, sorts: SortPair) {
        sort_rows(&mut self.per_ticket, sorts.ticket);
        sort_rows(&mut self.per_agent, sorts.agent);
    }
}

/// Dashboard state. `NoReceipts` is EXPLICIT (directory absent or empty) — the
/// render is [`no_receipts_text()`], never a table of zeros.
#[derive(Debug, PartialEq, Eq)]
pub enum MetricsState {
    NoReceipts,
    Loaded(MetricsModel),
}

// ---- sorting (column-click) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// `tokens_consumed.total` sum — the load-time default, descending.
    #[default]
    Tokens,
    Runs,
    Elapsed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub key: SortKey,
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
    pub ticket: Sort,
    pub agent: Sort,
}

/// Which table a header click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    Ticket,
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
