//! The estimate record, row, total and sort types.
//!
//! **Role:** `EstimateFile`, `ValidEstimate`, `EstimatedTokens`, `RawEstimates`, `EstimatedRow`,
//! `EstimatedTotals`, `EstimatesModel`, `EstimatesState` and the estimated sort types.
//! **Position:** part of `crate::execution_metrics::estimated`; read by the Metrics tab and the
//! ticket details.
//! **Signals & state:** none; plain data.
//! **Invariants:** the estimated types are distinct from the measured ones, so the two cannot be
//! added by accident.

use super::*;
// ---- estimate-file mirror (read-only) ----

/// Mirror of xtask `EstimateRecord` / `estimates.schema.json`.
/// `deny_unknown_fields` mirrors its `additionalProperties: false`; per-source
/// presence/absence is enforced in `validate_file`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EstimateFile {
    /// The ticket estimated; must equal the file stem.
    pub id: TicketId,
    /// How the estimate was made: `diff_loc` or `cohort_median`.
    pub source: String,
    /// Tokens per changed line.
    pub factor: u64,
    /// The estimated token count.
    pub tokens_estimated: u64,
    /// When the estimate was planned, RFC 3339 UTC.
    pub generated_at: String,
    #[serde(default)]
    /// Changed lines over the counted commits; `diff_loc` only.
    pub loc_changed: Option<u64>,
    #[serde(default)]
    /// The commits whose changed lines were counted; `diff_loc` only.
    pub derived_from_shas: Option<Vec<String>>,
    #[serde(default)]
    /// The widened cohort the median was taken over; `cohort_median` only.
    pub cohort: Option<CohortKey>,
    #[serde(default)]
    /// How many tickets the cohort held; `cohort_median` only.
    pub cohort_size: Option<u64>,
}

/// Typed per-source inputs of a VALIDATED estimate — every method records its
/// inputs (recalibration is regeneration, never untraceable mutation), and this
/// enum is those inputs, ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Changed lines times the factor.
    DiffLoc {
        /// Changed lines over the counted commits.
        loc_changed: u64,
        /// How many commits were counted.
        shas: usize,
    },
    /// The median of a widened cohort of `diff_loc` estimates.
    CohortMedian {
        /// The cohort key as displayed (`class=feature` and so on).
        key: String,
        /// How many tickets the cohort held.
        size: u64,
    },
}

impl Source {
    /// The source name as written in the file.
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::DiffLoc { .. } => "diff_loc",
            Source::CohortMedian { .. } => "cohort_median",
        }
    }
}

/// One validated estimate, ready to aggregate and to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidEstimate {
    /// The ticket estimated.
    pub id: TicketId,
    /// Tokens per changed line.
    pub factor: u64,
    /// The estimated token count.
    pub tokens_estimated: u64,
    /// When the estimate was planned, RFC 3339 UTC.
    pub generated_at: String,
    /// The method and its recorded inputs.
    pub source: Source,
}

/// Estimated token totals use a distinct type so they cannot be implicitly added
/// to measured receipt totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct EstimatedTokens(pub u64);

// ---- scan ----

/// Validated estimate records and named errors from a worker read. Aggregation
/// joins these records to the separately loaded corpus through build_state.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RawEstimates {
    /// `.ai/tickets/estimates/` exists on disk.
    pub present: bool,
    /// The valid estimates, in file-name order.
    pub records: Vec<ValidEstimate>,
    /// Malformed files, load order — excluded from every sum, listed verbatim.
    pub errors: Vec<ErrorRow>,
}

// ---- aggregation (pure) ----

/// One aggregated ESTIMATED table row (per class or per domain — estimates
/// carry no agent, so the measured dashboard's per-agent axis has no estimated
/// counterpart). Display strings are precomputed; the paint path never formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstimatedRow {
    /// The bucket: a class or a domain.
    pub key: String,
    /// Estimate files (one per ticket id by construction).
    pub tickets: u64,
    /// Σ `tokens_estimated` — the newtype, never a measured u64.
    pub tokens: EstimatedTokens,
    /// How many estimates came from changed lines.
    pub diff_loc: u64,
    /// How many estimates came from a cohort median.
    pub cohort_median: u64,
    /// The ticket count as displayed.
    pub tickets_str: String,
    /// The token sum as displayed.
    pub tokens_str: String,
    /// The changed-line count as displayed.
    pub diff_loc_str: String,
    /// The cohort-median count as displayed.
    pub cohort_str: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The estimated panel's grand totals.
pub struct EstimatedTotals {
    /// Valid estimate files.
    pub files: u64,
    /// The estimated token sum.
    pub tokens: EstimatedTokens,
    /// Estimates from changed lines.
    pub diff_loc: u64,
    /// Estimates from a cohort median.
    pub cohort_median: u64,
    /// Distinct classes.
    pub classes: usize,
    /// Distinct domains.
    pub domains: usize,
    /// Precomputed headline — the ESTIMATED strip, structurally separate from
    /// the measured strip; with zero valid files it says so.
    pub strip: String,
}

#[derive(Debug, PartialEq, Eq)]
/// The estimated panel: per-ticket details, per-class and per-domain rows, error rows and totals.
pub struct EstimatesModel {
    /// Per-ticket estimate details — the detail-panel "tokens (estimated)" row.
    pub by_id: BTreeMap<TicketId, EstimateDetail>,
    /// Sorted by the active sort (tokens desc on load).
    pub per_class: Vec<EstimatedRow>,
    /// Sorted by the active sort (tokens desc on load).
    pub per_domain: Vec<EstimatedRow>,
    /// Malformed files, load order — excluded from every sum, listed verbatim.
    pub errors: Vec<ErrorRow>,
    /// The grand totals.
    pub grand: EstimatedTotals,
}

impl EstimatesModel {
    /// Re-sorts both tables by the given selections.
    pub fn apply_sort(&mut self, sorts: EstimatedSortPair) {
        sort_rows(&mut self.per_class, sorts.class);
        sort_rows(&mut self.per_domain, sorts.domain);
    }
}

/// Estimated-panel state. `NoEstimates` is EXPLICIT (directory absent or
/// empty) — the render is [`no_estimates_text()`], never a table of zeros.
#[derive(Debug, PartialEq, Eq)]
pub enum EstimatesState {
    /// No estimate files: the panel shows the explicit empty state.
    NoEstimates,
    /// The aggregated estimates.
    Loaded(EstimatesModel),
}

// ---- sorting (column-click; estimated-only types) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// The column an estimated table sorts by.
pub enum EstimatedSortKey {
    /// Σ `tokens_estimated` — the load-time default, descending.
    #[default]
    Tokens,
    /// Estimate files per bucket.
    Tickets,
    /// Estimates from changed lines.
    DiffLoc,
    /// Estimates from a cohort median.
    CohortMedian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// One estimated table's sort: a column and a direction.
pub struct EstimatedSort {
    /// The sorted column.
    pub key: EstimatedSortKey,
    /// True for descending.
    pub desc: bool,
}

impl EstimatedSort {
    /// Header-click rule: same column flips direction, a new column starts desc.
    pub fn toggled(self, key: EstimatedSortKey) -> EstimatedSort {
        EstimatedSort {
            key,
            desc: if self.key == key { !self.desc } else { true },
        }
    }
}

/// Independent sort selections for the two estimated tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EstimatedSortPair {
    /// The per-class table's sort.
    pub class: EstimatedSort,
    /// The per-domain table's sort.
    pub domain: EstimatedSort,
}

/// Which estimated table a header click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimatedTableKind {
    /// The per-class table.
    Class,
    /// The per-domain table.
    Domain,
}

impl Default for EstimatedSort {
    fn default() -> Self {
        Self {
            key: EstimatedSortKey::Tokens,
            desc: true,
        }
    }
}
