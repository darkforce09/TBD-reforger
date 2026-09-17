//! Token usage.

use super::*;
use anyhow::Context;

// ── CLI usage parsing — the two RECORDED dialects ──────────────────────────────────────
//
// Pinned by fixtures in `tools_v2/ticket-engine/tests/fixtures/execution_receipts/` (recorded output, not guessed):
//   - `slice_run_cursor_agent.json` — `agent --output-format json`:
//     `usage.inputTokens` / `outputTokens` / `cacheReadTokens` / `cacheWriteTokens`,
//     optional `reasoningTokens`, optional `totalTokens`.
//   - `slice_run_claude_print.json` — `claude --print --output-format json`:
//     `usage.input_tokens` / `output_tokens` / `cache_read_input_tokens` /
//     `cache_creation_input_tokens`.
// Anything else is an unknown dialect and FAILS CLOSED — never coerced to zeros.
pub(super) fn u64_key(usage: &Value, key: &str) -> Result<Option<u64>> {
    match usage.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .with_context(|| format!("usage.{key} is not a non-negative integer: {v}")),
    }
}

pub(super) fn require_u64(usage: &Value, key: &str) -> Result<u64> {
    u64_key(usage, key)?.with_context(|| format!("usage object is missing {key}"))
}

/// Extract `tokens_consumed` from an agent CLI's final JSON, or fail closed.
pub fn parse_tokens_from_cli_json(cli: &Value) -> Result<TokensConsumed> {
    let usage = match cli.get("usage") {
        Some(u) if u.is_object() => u,
        // Cursor SDK builds have also emitted the camelCase keys at top level.
        _ if cli.get("inputTokens").is_some() => cli,
        _ => bail!(
            "agent CLI JSON has no usage object — the run FAILED; \
             refusing to invent tokens_consumed: 0"
        ),
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
        bail!(
            "usage object matches neither recorded dialect \
             (inputTokens… / input_tokens…) — refusing to guess"
        );
    };
    let total = input + output + cache_read + cache_write;
    // A reported total may be the four-way sum or (some Cursor builds) sum + reasoning.
    // Anything else is dialect drift and must be LOUD, not silently reconciled.
    if let Some(reported) = u64_key(usage, "totalTokens")? {
        let with_reasoning = total + reasoning.unwrap_or(0);
        if reported != total && reported != with_reasoning {
            bail!(
                "usage.totalTokens ({reported}) matches neither input+output+cache_read+\
                 cache_write ({total}) nor that sum plus reasoning ({with_reasoning}) — \
                 recorded dialect drifted, refusing to guess"
            );
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
