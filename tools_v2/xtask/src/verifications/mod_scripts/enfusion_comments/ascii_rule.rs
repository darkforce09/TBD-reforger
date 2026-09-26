//! ECM-1: an Enfusion script is ASCII only, string literals included.
//!
//! **Role:** reports every line that holds a character outside ASCII.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:** one finding per offending line, naming the first offending column (1-based,
//! in characters) and code point; a byte sequence that is not UTF-8 reads as U+FFFD and is
//! reported like any other non-ASCII character.

use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};

/// Reports every non-ASCII line of `script`.
///
/// Returns one [`Finding`] per offending line; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (number, raw) in script.raw_lines() {
        let Some((column, c)) = raw.chars().enumerate().find(|(_, c)| !c.is_ascii()) else {
            continue;
        };
        let count = raw.chars().filter(|c| !c.is_ascii()).count();
        findings.push(Finding::new(
            RuleId::AsciiOnly,
            number,
            format!(
                "non-ASCII U+{:04X} at column {} ({count} on the line); write -- or -> instead",
                c as u32,
                column + 1
            ),
        ));
    }
    findings
}
