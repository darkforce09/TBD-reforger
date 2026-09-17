//! Model.

use super::*;
use anyhow::Context;

/// Receipt tree, relative to the repo root. Deliberately OUTSIDE the ticket TOMLs and the
/// wave lock — parallel lands touch disjoint `<id>/` subtrees and never a shared file.
pub const METRICS_DIR_REL: &str = ".ai/tickets/metrics";

/// The committed schema every run file must satisfy.
pub const METRICS_SCHEMA_REL: &str = ".ai/tickets/metrics.schema.json";

pub fn metrics_root(root: &Path) -> PathBuf {
    root.join(METRICS_DIR_REL)
}

/// The required token observation. `total` is ALWAYS the four-way sum; `reasoning` is a
/// sibling observation (Cursor `reasoningTokens`) and is NEVER summed into `total`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TokensConsumed {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
}

impl TokensConsumed {
    pub fn validate(&self) -> Result<()> {
        let sum = self
            .input
            .checked_add(self.output)
            .and_then(|s| s.checked_add(self.cache_read))
            .and_then(|s| s.checked_add(self.cache_write))
            .context("token sum overflow")?;
        if sum != self.total {
            bail!(
                "tokens_consumed.total ({}) != input+output+cache_read+cache_write ({sum})",
                self.total
            );
        }
        Ok(())
    }
}

/// One run receipt. Field optionality mirrors `.ai/tickets/metrics.schema.json` exactly:
/// the producer knows `id`/`agent`/`started`/`tokens_consumed` for certain; `finished`,
/// `outcome` and `git_sha` are stamps (the producer writes its own, `platform wave land`
/// overwrites them with the land's). Elapsed is DERIVED (`finished − started`) at query
/// time — deliberately not stored, so it can never disagree with the timestamps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    pub id: String,
    pub agent: String,
    pub started: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    pub tokens_consumed: TokensConsumed,
}

/// RFC 3339 UTC parse with the SAME acceptance rule as the ticket lifecycle stamps
/// (T-913.1): reuse `tbd_tickets` validation, then reparse for the arithmetic.
pub(super) fn parse_utc(field: &str, value: &str) -> Result<OffsetDateTime> {
    crate::validate_rfc3339_utc(field, value).map_err(|e| anyhow::anyhow!("{e}"))?;
    OffsetDateTime::parse(value, &Rfc3339)
        .with_context(|| format!("{field} {value:?} failed to reparse"))
}

/// Semantic invariants the JSON Schema cannot express: canonical UTC timestamps,
/// `finished >= started`, and the token-sum rule.
pub fn validate_record(rec: &RunRecord) -> Result<()> {
    if rec.id.trim().is_empty() {
        bail!("id must be non-empty");
    }
    if rec.agent.trim().is_empty() {
        bail!("agent must be non-empty");
    }
    let started = parse_utc("started", &rec.started)?;
    if let Some(fin) = rec.finished.as_deref() {
        let finished = parse_utc("finished", fin)?;
        if finished < started {
            bail!("finished {fin} is before started {}", rec.started);
        }
    }
    rec.tokens_consumed.validate()?;
    Ok(())
}

/// `finished − started` in whole seconds, `None` when the run has no `finished` stamp.
pub fn elapsed_sec(rec: &RunRecord) -> Result<Option<u64>> {
    let Some(fin) = rec.finished.as_deref() else {
        return Ok(None);
    };
    let s = parse_utc("started", &rec.started)?;
    let f = parse_utc("finished", fin)?;
    let secs = (f - s).whole_seconds();
    if secs < 0 {
        bail!("finished {fin} is before started {}", rec.started);
    }
    Ok(Some(secs as u64))
}
