//! Token counts from an agent CLI's final JSON.
//!
//! **Role:** turns the final JSON an agent CLI prints into the
//! [`ticket_manager_client::TokenCounts`] a run receipt records ([`parse_tokens_from_cli_json`]),
//! or the [`Error`] that says why it cannot.
//! **Position:** `platform slice-run` calls it on the captured output before it records a receipt
//! with `ttm record-run`.
//! **Signals & state:** none; pure functions.
//! **Invariants:** only the two recorded dialects parse, the Cursor agent's camelCase keys and
//! Claude's snake_case keys; a missing usage object, an unknown dialect, a count that is not a
//! non-negative integer, a four-way sum that overflows, or a reported total that is neither the
//! four-way sum nor that sum plus reasoning is an error, never a zero count.

use serde_json::Value;
use ticket_manager_client::TokenCounts;

/// Why an agent CLI's final JSON yields no token counts; the text says what was wrong.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(String);

// ── CLI usage parsing — the two RECORDED dialects ──────────────────────────────────────
//
// The two dialects, as the agent CLIs print them:
//   - `agent --output-format json`:
//     `usage.inputTokens` / `outputTokens` / `cacheReadTokens` / `cacheWriteTokens`,
//     optional `reasoningTokens`, optional `totalTokens`.
//   - `claude --print --output-format json`:
//     `usage.input_tokens` / `output_tokens` / `cache_read_input_tokens` /
//     `cache_creation_input_tokens`.
// Anything else is an unknown dialect and FAILS CLOSED — never coerced to zeros.

/// `usage[key]` as a count: `None` when the key is absent, an error when it is present but
/// not a non-negative integer.
fn u64_key(usage: &Value, key: &str) -> Result<Option<u64>, Error> {
    match usage.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .ok_or_else(|| Error(format!("usage.{key} is not a non-negative integer: {v}"))),
    }
}

/// `usage[key]` as a count that must be present.
fn require_u64(usage: &Value, key: &str) -> Result<u64, Error> {
    u64_key(usage, key)?.ok_or_else(|| Error(format!("usage object is missing {key}")))
}

/// Extract the token counts from an agent CLI's final JSON, or fail closed.
pub fn parse_tokens_from_cli_json(cli: &Value) -> Result<TokenCounts, Error> {
    let usage = match cli.get("usage") {
        Some(u) if u.is_object() => u,
        // Cursor SDK builds have also emitted the camelCase keys at top level.
        _ if cli.get("inputTokens").is_some() => cli,
        _ => {
            return Err(Error(
                "agent CLI JSON has no usage object — the run FAILED; refusing to invent \
                 tokens_consumed: 0"
                    .to_string(),
            ));
        }
    };
    let camel = usage.get("inputTokens").is_some();
    let snake = usage.get("input_tokens").is_some();
    let (input, output, cache_read, cache_write, reasoning) = if camel {
        (
            require_u64(usage, "inputTokens")?,
            require_u64(usage, "outputTokens")?,
            u64_key(usage, "cacheReadTokens")?.unwrap_or(0),
            u64_key(usage, "cacheWriteTokens")?.unwrap_or(0),
            u64_key(usage, "reasoningTokens")?,
        )
    } else if snake {
        (
            require_u64(usage, "input_tokens")?,
            require_u64(usage, "output_tokens")?,
            u64_key(usage, "cache_read_input_tokens")?.unwrap_or(0),
            u64_key(usage, "cache_creation_input_tokens")?.unwrap_or(0),
            u64_key(usage, "reasoning_tokens")?,
        )
    } else {
        return Err(Error(
            "usage object matches neither recorded dialect (inputTokens… / input_tokens…) — \
             refusing to guess"
                .to_string(),
        ));
    };
    let total = input
        .checked_add(output)
        .and_then(|sum| sum.checked_add(cache_read))
        .and_then(|sum| sum.checked_add(cache_write))
        .ok_or_else(|| Error("token sum overflow".to_string()))?;
    // A reported total may be the four-way sum or (some Cursor builds) sum + reasoning.
    // Anything else is dialect drift and must be LOUD, not silently reconciled.
    if let Some(reported) = u64_key(usage, "totalTokens")? {
        let with_reasoning = total.saturating_add(reasoning.unwrap_or(0));
        if reported != total && reported != with_reasoning {
            return Err(Error(format!(
                "usage.totalTokens ({reported}) matches neither input+output+cache_read+\
                 cache_write ({total}) nor that sum plus reasoning ({with_reasoning}) — \
                 recorded dialect drifted, refusing to guess"
            )));
        }
    }
    Ok(TokenCounts {
        input,
        output,
        cache_read,
        cache_write,
        reasoning,
    })
}

#[cfg(test)]
#[path = "tests/token_usage_tests.rs"]
mod tests;
