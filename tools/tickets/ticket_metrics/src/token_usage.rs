//! Token counts from an agent CLI's final JSON.
//!
//! **Role:** turns the final JSON an agent CLI prints into a [`crate::TokensConsumed`]
//! ([`parse_tokens_from_cli_json`]).
//! **Position:** xtask's `platform slice-run` calls it on the captured output before writing a
//! receipt.
//! **Signals & state:** none; pure functions.
//! **Invariants:** only the two recorded dialects parse, the Cursor agent's camelCase keys and
//! Claude's snake_case keys; a missing usage object, an unknown dialect or a reported total that
//! is neither the four-way sum nor that sum plus reasoning is an error, never a zero count.

use crate::TokensConsumed;
use crate::error::{Error, Result, ResultExt};
use serde_json::Value;

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
pub(super) fn u64_key(usage: &Value, key: &str) -> Result<Option<u64>> {
    match usage.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .with_context(|| format!("usage.{key} is not a non-negative integer: {v}")),
    }
}

/// `usage[key]` as a count that must be present.
pub(super) fn require_u64(usage: &Value, key: &str) -> Result<u64> {
    u64_key(usage, key)?.with_context(|| format!("usage object is missing {key}"))
}

/// Extract `tokens_consumed` from an agent CLI's final JSON, or fail closed.
pub fn parse_tokens_from_cli_json(cli: &Value) -> Result<TokensConsumed> {
    let usage = match cli.get("usage") {
        Some(u) if u.is_object() => u,
        // Cursor SDK builds have also emitted the camelCase keys at top level.
        _ if cli.get("inputTokens").is_some() => cli,
        _ => {
            return Err(Error::msg(
                "agent CLI JSON has no usage object — the run FAILED; \
             refusing to invent tokens_consumed: 0",
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
        return Err(Error::msg(
            "usage object matches neither recorded dialect \
             (inputTokens… / input_tokens…) — refusing to guess",
        ));
    };
    let total = input + output + cache_read + cache_write;
    // A reported total may be the four-way sum or (some Cursor builds) sum + reasoning.
    // Anything else is dialect drift and must be LOUD, not silently reconciled.
    if let Some(reported) = u64_key(usage, "totalTokens")? {
        let with_reasoning = total + reasoning.unwrap_or(0);
        if reported != total && reported != with_reasoning {
            return Err(Error::msg(format!(
                "usage.totalTokens ({reported}) matches neither input+output+cache_read+\
                 cache_write ({total}) nor that sum plus reasoning ({with_reasoning}) — \
                 recorded dialect drifted, refusing to guess"
            )));
        }
    }
    let tokens = TokensConsumed {
        input,
        output,
        cache_read,
        cache_write,
        total,
        reasoning,
    };
    tokens.validate()?;
    Ok(tokens)
}
