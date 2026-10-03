//! The run receipt and its semantic check.
//!
//! **Role:** declares [`RunRecord`] and [`TokensConsumed`], the typed form of one file under
//! `.ai/tickets/metrics/<id>/`, the checks the JSON Schema cannot express ([`validate_record`],
//! [`TokensConsumed::validate`]) and the derived elapsed time ([`elapsed_sec`]).
//! **Position:** the receipt writer, the land stamp, the summary and the `ticket check` walker
//! of this crate build on it; xtask's `platform slice-run` builds a [`RunRecord`] directly.
//! **Signals & state:** none; plain data and pure functions.
//! **Invariants:** `tokens_consumed.total` is the four-way sum of input, output, cache read and
//! cache write, and `reasoning` is never added to it; every stamp passes the same RFC 3339 UTC
//! rule as the ticket lifecycle stamps; `finished` is never before `started`; elapsed time is
//! computed from the two stamps and never stored, so it cannot disagree with them.

use crate::error::{Error, Result, ResultExt};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ticket_model::TicketId;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The receipt tree of the checkout at `root`: `<root>/.ai/tickets/metrics`
/// ([`repository_layout::METRICS_DIR`]). The folder may not exist yet.
pub fn metrics_root(root: &Path) -> PathBuf {
    root.join(repository_layout::METRICS_DIR)
}

/// The token counts one agent run consumed, the `tokens_consumed` object of a receipt. `total`
/// is always the four-way sum; `reasoning` is a sibling observation (the Cursor agent's
/// `reasoningTokens`) and is never summed into `total`. Unknown keys fail deserialisation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TokensConsumed {
    /// Uncached prompt tokens sent to the model (`input`).
    pub input: u64,
    /// Tokens the model generated (`output`).
    pub output: u64,
    /// Prompt tokens served from the provider's cache (`cache_read`).
    pub cache_read: u64,
    /// Prompt tokens written into the provider's cache (`cache_write`).
    pub cache_write: u64,
    /// `input + output + cache_read + cache_write`; any other value is refused (`total`).
    pub total: u64,
    /// Reasoning tokens when the agent reports them, recorded beside `total` and never added to
    /// it; `None` (key omitted) when the agent does not report them (`reasoning`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
}

impl TokensConsumed {
    /// Checks the four-way sum rule. An error names both `total` and the sum, or reports that
    /// the sum overflows `u64`.
    pub fn validate(&self) -> Result<()> {
        let sum = self
            .input
            .checked_add(self.output)
            .and_then(|s| s.checked_add(self.cache_read))
            .and_then(|s| s.checked_add(self.cache_write))
            .context("token sum overflow")?;
        if sum != self.total {
            return Err(Error::msg(format!(
                "tokens_consumed.total ({}) != input+output+cache_read+cache_write ({sum})",
                self.total
            )));
        }
        Ok(())
    }
}

/// One run receipt, the JSON file `.ai/tickets/metrics/<id>/<started>-<sha>.json`. Field
/// optionality mirrors `.ai/tickets/metrics.schema.json` exactly: the producer knows `id`,
/// `agent`, `started` and `tokens_consumed` for certain; `finished`, `outcome` and `git_sha` are
/// stamps (the producer writes its own, `platform wave land` overwrites them with the land's).
/// Elapsed time is computed as `finished − started` when asked for ([`elapsed_sec`]) and never
/// stored, so it can never disagree with the timestamps. Unknown keys fail deserialisation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    /// The ticket the run worked on; serialises as its bare id string and must equal the name
    /// of the folder the file sits in (`id`).
    pub id: TicketId,
    /// The agent that ran the ticket: `platform slice-run` records the basename of the invoked
    /// program, such as `agent` or `claude`; never blank (`agent`).
    pub agent: String,
    /// When the run started, RFC 3339 UTC (`started`).
    pub started: String,
    /// When the run finished, RFC 3339 UTC and not before `started`; `None` (key omitted) while
    /// no one has stamped it (`finished`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished: Option<String>,
    /// How the run ended: `ran` as `platform slice-run` writes it, `landed` after the land
    /// stamp; `None` (key omitted) when the producer wrote none (`outcome`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    /// The commit the run produced, or the land commit once stamped; `None` (key omitted) when
    /// unknown, which names the file `…-nosha.json` (`git_sha`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    /// The tokens the run consumed (`tokens_consumed`).
    pub tokens_consumed: TokensConsumed,
}

/// Parses an RFC 3339 UTC stamp under the same acceptance rule as the ticket lifecycle stamps
/// ([`time_source::validate_rfc3339_utc`]), then reparses it for the arithmetic.
pub(super) fn parse_utc(field: &str, value: &str) -> Result<OffsetDateTime> {
    time_source::validate_rfc3339_utc(field, value)?;
    OffsetDateTime::parse(value, &Rfc3339)
        .with_context(|| format!("{field} {value:?} failed to reparse"))
}

/// Checks the invariants the JSON Schema cannot express: a non-blank `id` and `agent`,
/// canonical UTC stamps, `finished >= started`, and the token-sum rule. The error names the
/// first rule the record breaks.
pub fn validate_record(rec: &RunRecord) -> Result<()> {
    if rec.id.as_str().trim().is_empty() {
        return Err(Error::msg("id must be non-empty"));
    }
    if rec.agent.trim().is_empty() {
        return Err(Error::msg("agent must be non-empty"));
    }
    let started = parse_utc("started", &rec.started)?;
    if let Some(fin) = rec.finished.as_deref() {
        let finished = parse_utc("finished", fin)?;
        if finished < started {
            return Err(Error::msg(format!(
                "finished {fin} is before started {}",
                rec.started
            )));
        }
    }
    rec.tokens_consumed.validate()?;
    Ok(())
}

/// `finished − started` in whole seconds, `None` when the run has no `finished` stamp. An error
/// means a stamp is not RFC 3339 UTC or `finished` is before `started`.
pub fn elapsed_sec(rec: &RunRecord) -> Result<Option<u64>> {
    let Some(fin) = rec.finished.as_deref() else {
        return Ok(None);
    };
    let s = parse_utc("started", &rec.started)?;
    let f = parse_utc("finished", fin)?;
    let secs = (f - s).whole_seconds();
    if secs < 0 {
        return Err(Error::msg(format!(
            "finished {fin} is before started {}",
            rec.started
        )));
    }
    Ok(Some(secs as u64))
}
