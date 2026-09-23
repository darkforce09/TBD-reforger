use super::*;
// ---- estimate-file mirror (read-only) ----

/// Mirror of the schema's `cohort` object (xtask `CohortKey`): the WIDENED key
/// actually used — only the fields that constrained the cohort are present
/// (`{}` = all diff_loc tickets).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CohortKey {
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub layer: Option<String>,
}

/// Mirror of xtask `EstimateRecord` / `estimates.schema.json`.
/// `deny_unknown_fields` mirrors its `additionalProperties: false`; per-source
/// presence/absence is enforced in [`validate_file`].
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EstimateFile {
    pub id: String,
    pub source: String,
    pub factor: u64,
    pub tokens_estimated: u64,
    pub generated_at: String,
    #[serde(default)]
    pub loc_changed: Option<u64>,
    #[serde(default)]
    pub derived_from_shas: Option<Vec<String>>,
    #[serde(default)]
    pub cohort: Option<CohortKey>,
    #[serde(default)]
    pub cohort_size: Option<u64>,
}

/// Typed per-source inputs of a VALIDATED estimate — every method records its
/// inputs (recalibration is regeneration, never untraceable mutation), and this
/// enum is those inputs, ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    DiffLoc { loc_changed: u64, shas: usize },
    CohortMedian { key: String, size: u64 },
}

impl Source {
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
    pub id: String,
    pub factor: u64,
    pub tokens_estimated: u64,
    pub generated_at: String,
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
    pub key: String,
    /// Estimate files (one per ticket id by construction).
    pub tickets: u64,
    /// Σ `tokens_estimated` — the newtype, never a measured u64.
    pub tokens: EstimatedTokens,
    pub diff_loc: u64,
    pub cohort_median: u64,
    pub tickets_str: String,
    pub tokens_str: String,
    pub diff_loc_str: String,
    pub cohort_str: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstimatedTotals {
    pub files: u64,
    pub tokens: EstimatedTokens,
    pub diff_loc: u64,
    pub cohort_median: u64,
    pub classes: usize,
    pub domains: usize,
    /// Precomputed headline — the ESTIMATED strip, structurally separate from
    /// the measured strip; with zero valid files it says so.
    pub strip: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EstimatesModel {
    /// Per-ticket estimate details — the detail-panel "tokens (estimated)" row.
    pub by_id: BTreeMap<String, EstimateDetail>,
    /// Sorted by the active sort (tokens desc on load).
    pub per_class: Vec<EstimatedRow>,
    pub per_domain: Vec<EstimatedRow>,
    /// Malformed files, load order — excluded from every sum, listed verbatim.
    pub errors: Vec<ErrorRow>,
    pub grand: EstimatedTotals,
}

impl EstimatesModel {
    pub fn apply_sort(&mut self, sorts: EstimatedSortPair) {
        sort_rows(&mut self.per_class, sorts.class);
        sort_rows(&mut self.per_domain, sorts.domain);
    }
}

/// Estimated-panel state. `NoEstimates` is EXPLICIT (directory absent or
/// empty) — the render is [`no_estimates_text()`], never a table of zeros.
#[derive(Debug, PartialEq, Eq)]
pub enum EstimatesState {
    NoEstimates,
    Loaded(EstimatesModel),
}

// ---- sorting (column-click; estimated-only types) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EstimatedSortKey {
    /// Σ `tokens_estimated` — the load-time default, descending.
    #[default]
    Tokens,
    Tickets,
    DiffLoc,
    CohortMedian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EstimatedSort {
    pub key: EstimatedSortKey,
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
    pub class: EstimatedSort,
    pub domain: EstimatedSort,
}

/// Which estimated table a header click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimatedTableKind {
    Class,
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
