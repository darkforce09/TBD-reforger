//! The agent prompt a spec document carries.
//!
//! **Role:** extracts the fenced block under a spec's `## Claude Code prompt` heading.
//! **Position:** used by the `ticket prompt` verb (`cmd_prompt`).
//! **Signals & state:** none; pure functions over the document text.
//! **Invariants:** a document without the section, or a section without a fenced block, is an
//! error, never an empty prompt.

use crate::error::{Error, Result};
use regex::Regex;
use std::sync::LazyLock;

/// The `## Claude Code prompt` heading and the first fenced block after it.
static SECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?ms)^##\s+Claude Code prompt[^\n]*\n+(?:[^\n]*\n)*?```(?:\w*\n)?(.*?)```")
        .expect("the prompt section pattern compiles")
});

/// Any fenced block, for a prompt section whose fence the [`SECTION`] pattern does not reach.
static FENCED_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)```(?:\w*\n)?(.*?)```").expect("the fenced block pattern compiles")
});

/// The trimmed contents of the first fenced block under the `## Claude Code prompt` heading of
/// `markdown`.
///
/// # Errors
/// When the document has no such heading, or no fenced block follows it.
pub fn extract_prompt(markdown: &str) -> Result<String> {
    if let Some(c) = SECTION.captures(markdown) {
        return Ok(c[1].trim().to_string());
    }
    let idx = markdown
        .find("## Claude Code prompt")
        .ok_or_else(|| Error::msg("No '## Claude Code prompt' section found"))?;
    let rest = &markdown[idx..];
    let block = FENCED_BLOCK
        .captures(rest)
        .ok_or_else(|| Error::msg("No fenced code block in Claude Code prompt section"))?;
    Ok(block[1].trim().to_string())
}
