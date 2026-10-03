//! The estimate record and its semantic check.
//!
//! **Role:** declares [`EstimateRecord`] and [`CohortKey`], the typed form of one
//! `.ai/tickets/estimates/<id>.json`, the factor [`TOKENS_PER_LOC`], the tree's location
//! ([`estimates_root`]) and the checks the JSON Schema cannot express ([`validate_estimate`]).
//! **Position:** the planners, the storage and the check of this module build on it;
//! `ticket_registry`'s `stamp-sha` verb writes records through it.
//! **Signals & state:** none; plain data and pure functions.
//! **Invariants:** fields are declared in alphabetical order, which is the on-disk key order
//! because `serde_json` (`preserve_order`) writes struct declaration order; each source carries
//! only its own fields; a `diff_loc` estimate is exactly `loc_changed × factor`.

use super::*;
use crate::error::{Error, Result, ResultExt};

/// Tokens per changed line of code. The measurement behind the number, and the cohorts it was
/// taken over, live in [`ticket_model::repository::documentation::TOKEN_ESTIMATE_FACTOR_DOC`]; a test
/// asserts that document quotes this value verbatim, so the two cannot drift.
pub const TOKENS_PER_LOC: u64 = 150;

/// The estimate tree of the checkout at `root`: `<root>/.ai/tickets/estimates`
/// ([`repository_layout::ESTIMATES_DIR`]). The folder may not exist yet.
pub fn estimates_root(root: &Path) -> PathBuf {
    root.join(repository_layout::ESTIMATES_DIR)
}

/// The widened cohort key a `cohort_median` estimate actually used: only the fields that
/// constrained the cohort are present, and `{}` means every `diff_loc` ticket. Fields are
/// declared alphabetically because that is the on-disk key order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CohortKey {
    /// The ticket class the cohort shares; `None` (key omitted) when the cohort spans every
    /// class (`class`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// The scope domain the cohort shares; `None` (key omitted) when the cohort spans every
    /// domain (`domain`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// The scope layer the cohort shares; `None` (key omitted) when the cohort spans every
    /// layer (`layer`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

/// One token estimate, the JSON file `.ai/tickets/estimates/<id>.json`. Field optionality
/// mirrors `.ai/tickets/estimates.schema.json` exactly (`diff_loc` ⇒ `loc_changed` +
/// `derived_from_shas`; `cohort_median` ⇒ `cohort` + `cohort_size`; each source forbids the
/// other's fields). Fields are declared alphabetically because `serde_json`
/// (`preserve_order`) writes declaration order and the file contract is sorted keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EstimateRecord {
    /// The widened cohort the median was taken over; present only for `cohort_median`
    /// (`cohort`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort: Option<CohortKey>,
    /// How many `diff_loc` tickets the cohort held, at least 1; present only for
    /// `cohort_median` (`cohort_size`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort_size: Option<u64>,
    /// The commits whose changed lines were counted, each 7 to 40 lowercase hex characters, at
    /// least one; present only for `diff_loc` (`derived_from_shas`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from_shas: Option<Vec<String>>,
    /// Tokens per changed line used for the estimate; `ticket check` requires
    /// [`TOKENS_PER_LOC`] (`factor`).
    pub factor: u64,
    /// When the estimate was planned, RFC 3339 UTC (`generated_at`).
    pub generated_at: String,
    /// The ticket estimated; serialises as its bare id string and must equal the file stem
    /// (`id`).
    pub id: TicketId,
    /// Changed lines over the counted paths of `derived_from_shas`; present only for
    /// `diff_loc` (`loc_changed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loc_changed: Option<u64>,
    /// How the estimate was made: `diff_loc` or `cohort_median` (`source`).
    pub source: String,
    /// The estimated token count: `loc_changed × factor` for `diff_loc`, the cohort median for
    /// `cohort_median` (`tokens_estimated`).
    pub tokens_estimated: u64,
}

/// Checks the invariants the JSON Schema cannot (or should not solely) express: canonical UTC
/// `generated_at`, a non-zero `factor`, per-source field presence and absence, commit shapes,
/// and `tokens_estimated == loc_changed × factor` for `diff_loc`. The error names the first
/// rule the record breaks.
pub fn validate_estimate(rec: &EstimateRecord) -> Result<()> {
    if rec.id.as_str().trim().is_empty() {
        return Err(Error::msg("id must be non-empty"));
    }
    validate_rfc3339_utc("generated_at", &rec.generated_at)?;
    if rec.factor == 0 {
        return Err(Error::msg("factor must be >= 1"));
    }
    match rec.source.as_str() {
        "diff_loc" => {
            let loc = rec
                .loc_changed
                .context("source diff_loc requires loc_changed")?;
            let shas = rec
                .derived_from_shas
                .as_ref()
                .context("source diff_loc requires derived_from_shas")?;
            if shas.is_empty() {
                return Err(Error::msg(
                    "derived_from_shas must name at least one subject SHA",
                ));
            }
            for s in shas {
                if !is_sha_shaped(s) {
                    return Err(Error::msg(format!(
                        "derived_from_shas entry {s:?} is not 7-40 lowercase hex"
                    )));
                }
            }
            if rec.cohort.is_some() || rec.cohort_size.is_some() {
                return Err(Error::msg("source diff_loc carries no cohort fields"));
            }
            let expect = loc
                .checked_mul(rec.factor)
                .context("loc_changed x factor overflow")?;
            if rec.tokens_estimated != expect {
                return Err(Error::msg(format!(
                    "tokens_estimated ({}) != loc_changed ({loc}) x factor ({}) = {expect}",
                    rec.tokens_estimated, rec.factor
                )));
            }
        }
        "cohort_median" => {
            let size = rec
                .cohort_size
                .context("source cohort_median requires cohort_size")?;
            if size == 0 {
                return Err(Error::msg("cohort_size must be >= 1"));
            }
            if rec.cohort.is_none() {
                return Err(Error::msg(
                    "source cohort_median requires the cohort key (the WIDENED key actually used; {} = all diff_loc)",
                ));
            }
            if rec.loc_changed.is_some() || rec.derived_from_shas.is_some() {
                return Err(Error::msg(
                    "source cohort_median carries no diff_loc fields",
                ));
            }
        }
        other => {
            return Err(Error::msg(format!(
                "unknown source {other:?} (diff_loc|cohort_median)"
            )));
        }
    }
    Ok(())
}
