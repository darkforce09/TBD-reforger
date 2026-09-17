//! Model.

use super::*;
use anyhow::Context;

/// Tokens per LOC changed — the declared constant pending calibration. Derivation
/// (measured ONCE, T-915/T-916 program: ~2.4M subagent output-tokens over ~16k LOC)
/// lives in [`FACTOR_DOC_REL`]; a test asserts the doc quotes this value verbatim.
pub const TOKENS_PER_LOC: u64 = 150;

/// The factor's document of record.
pub const FACTOR_DOC_REL: &str = "docs/platform/token_estimate_factor.md";

/// Estimate tree, relative to the repo root — deliberately OUTSIDE `metrics/`.
pub const ESTIMATES_DIR_REL: &str = ".ai/tickets/estimates";

/// The committed schema every estimate file must satisfy.
pub const ESTIMATES_SCHEMA_REL: &str = ".ai/tickets/estimates.schema.json";

pub fn estimates_root(root: &Path) -> PathBuf {
    root.join(ESTIMATES_DIR_REL)
}

/// The WIDENED cohort key actually used — only the fields that constrained the
/// cohort are present (`{}` = all diff_loc tickets). Field order is alphabetical
/// on purpose: it is the on-disk key order (see the module header).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CohortKey {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

/// One token estimate. Field optionality mirrors `.ai/tickets/estimates.schema.json`
/// exactly (`diff_loc` ⇒ `loc_changed` + `derived_from_shas`; `cohort_median` ⇒
/// `cohort` + `cohort_size`; each source forbids the other's fields). Fields are
/// declared ALPHABETICALLY on purpose — serde_json (`preserve_order`) emits struct
/// declaration order, and the file contract is sorted keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EstimateRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort: Option<CohortKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from_shas: Option<Vec<String>>,
    pub factor: u64,
    pub generated_at: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loc_changed: Option<u64>,
    pub source: String,
    pub tokens_estimated: u64,
}

/// Semantic invariants the JSON Schema cannot (or should not solely) express:
/// canonical UTC `generated_at`, per-source field presence/absence, SHA shapes,
/// and the `tokens_estimated == loc_changed × factor` arithmetic for `diff_loc`.
pub fn validate_estimate(rec: &EstimateRecord) -> Result<()> {
    if rec.id.trim().is_empty() {
        bail!("id must be non-empty");
    }
    validate_rfc3339_utc("generated_at", &rec.generated_at).map_err(anyhow::Error::msg)?;
    if rec.factor == 0 {
        bail!("factor must be >= 1");
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
                bail!("derived_from_shas must name at least one subject SHA");
            }
            for s in shas {
                if !is_sha_shaped(s) {
                    bail!("derived_from_shas entry {s:?} is not 7-40 lowercase hex");
                }
            }
            if rec.cohort.is_some() || rec.cohort_size.is_some() {
                bail!("source diff_loc carries no cohort fields");
            }
            let expect = loc
                .checked_mul(rec.factor)
                .context("loc_changed x factor overflow")?;
            if rec.tokens_estimated != expect {
                bail!(
                    "tokens_estimated ({}) != loc_changed ({loc}) x factor ({}) = {expect}",
                    rec.tokens_estimated,
                    rec.factor
                );
            }
        }
        "cohort_median" => {
            let size = rec
                .cohort_size
                .context("source cohort_median requires cohort_size")?;
            if size == 0 {
                bail!("cohort_size must be >= 1");
            }
            if rec.cohort.is_none() {
                bail!(
                    "source cohort_median requires the cohort key (the WIDENED key actually used; {{}} = all diff_loc)"
                );
            }
            if rec.loc_changed.is_some() || rec.derived_from_shas.is_some() {
                bail!("source cohort_median carries no diff_loc fields");
            }
        }
        other => bail!("unknown source {other:?} (diff_loc|cohort_median)"),
    }
    Ok(())
}
