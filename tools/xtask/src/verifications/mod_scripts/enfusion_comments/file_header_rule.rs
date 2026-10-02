//! ECM-2: every Enfusion script opens with its `/** ... */` file header.
//!
//! **Role:** checks that line 1 opens a `/**` block and that the block carries `@file <name>.c`,
//! `@brief`, `Role:`, `Position:`, `State:` and `Invariants:`, each with text.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:**
//! - The header must be the first line; nothing, not even a blank line, precedes it.
//! - `@file` must name the file exactly, extension included.
//! - A missing header yields one finding on line 1, not one per missing field.

use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};

/// The labelled fields the header must carry, each followed by text.
const HEADER_FIELDS: [&str; 4] = ["Role:", "Position:", "State:", "Invariants:"];

/// Checks the file header of `script`.
///
/// Returns the findings, empty when the header is complete; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let first = script.line(1).map(|line| line.raw.trim_end()).unwrap_or("");
    if first != "/**" {
        return vec![Finding::new(
            RuleId::FileHeader,
            1,
            "the file does not open with a /** header block on line 1",
        )];
    }
    let header: Vec<(usize, &str)> = header_lines(script);
    let mut findings = Vec::new();
    let expected = &script.file_name;
    match tag_value(&header, "@file") {
        None => findings.push(Finding::new(
            RuleId::FileHeader,
            1,
            "the header has no @file line",
        )),
        Some((line, value)) if value != expected => findings.push(Finding::new(
            RuleId::FileHeader,
            line,
            format!("@file names `{value}`; the file is `{expected}`"),
        )),
        Some(_) => {}
    }
    match tag_value(&header, "@brief") {
        Some((_, value)) if !value.is_empty() => {}
        _ => findings.push(Finding::new(
            RuleId::FileHeader,
            1,
            "the header has no @brief text",
        )),
    }
    for field in HEADER_FIELDS {
        let present = header.iter().any(|(_, text)| {
            text.split_once(field).is_some_and(|(before, after)| {
                starts_label(before) && !until_next_label(after).trim().is_empty()
            })
        });
        if !present {
            findings.push(Finding::new(
                RuleId::FileHeader,
                1,
                format!("the header has no `{field}` with text"),
            ));
        }
    }
    findings
}

/// The lines of the opening block comment, each without its leading `*`, up to its `*/`.
fn header_lines(script: &CheckedScript) -> Vec<(usize, &str)> {
    let mut header = Vec::new();
    for (number, raw) in script.raw_lines().skip(1) {
        let trimmed = raw.trim();
        if trimmed.starts_with("*/") {
            break;
        }
        let text = trimmed.strip_prefix('*').unwrap_or(trimmed).trim();
        header.push((number, text));
        if trimmed.ends_with("*/") {
            break;
        }
    }
    header
}

/// The line and value of the first header line that starts with `tag`.
fn tag_value<'a>(header: &[(usize, &'a str)], tag: &str) -> Option<(usize, &'a str)> {
    header.iter().find_map(|(number, text)| {
        let rest = text.strip_prefix(tag)?;
        (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| (*number, rest.trim()))
    })
}

/// The text after a label up to the next label on the same line.
fn until_next_label(after: &str) -> &str {
    HEADER_FIELDS
        .iter()
        .filter_map(|label| after.find(label))
        .min()
        .map_or(after, |end| &after[..end])
}

/// Whether the text before a field label ends where a label may start (line start or a blank).
fn starts_label(before: &str) -> bool {
    before.is_empty() || before.ends_with(char::is_whitespace)
}
