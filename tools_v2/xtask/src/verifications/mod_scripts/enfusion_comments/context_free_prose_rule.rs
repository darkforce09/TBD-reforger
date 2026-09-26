//! ECM-8: comments are present tense and context-free.
//!
//! **Role:** scans comment text only for ticket ids, dates, history words, work markers, delivery
//! vocabulary, separator lines, commented-out code and cross-repository `file:line` references.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:**
//! - Code and string literals are never scanned, so a string such as `"slice"` is not a finding.
//! - Words match whole words, case-insensitively; one finding per comment line and lexicon entry.

use regex::Regex;

use super::super::enfusion_script_lexer::CommentMarker;
use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};

/// Identifier patterns a comment must not carry: ticket ids, Enfusion issue ids, calendar dates.
pub(crate) const BANNED_IDENTIFIER_PATTERNS: [(&str, &str); 3] = [
    (r"\bT-[0-9]{3,}(\.[0-9]+)*\b", "ticket id"),
    (r"\bENF-[0-9]+\b", "issue id"),
    (r"\b20[0-9]{2}-[0-9]{2}-[0-9]{2}\b", "date"),
];

/// Whole words and phrases a comment must not carry, matched case-insensitively.
pub(crate) const BANNED_WORDS: [&str; 15] = [
    "previously",
    "formerly",
    "used to",
    "no longer",
    "reworked",
    "rewritten",
    "split out of",
    "ported",
    "legacy",
    "TODO",
    "FIXME",
    "HACK",
    "wave",
    "slice",
    "lane",
];

/// A comment that is only a rule of punctuation, or a title fenced by such rules.
const SEPARATOR: &str = r"^([-=*#~_/+]{3,}|[-=*#~_/+]{3,}\s.*\s[-=*#~_/+]{3,})$";

/// A comment that reads as a disabled statement: a lone brace, a control header, a `return`, a
/// call, an assignment or a declaration, each ending in `;` where a statement would.
const COMMENTED_OUT_CODE: &str = concat!(
    r#"^(\{|\}|(if|for|foreach|while|switch)\s*\(.*|return(\s+[\w.()!"]+)?\s*;"#,
    r"|[A-Za-z_][\w.]*\s*\(.*\)\s*;|[A-Za-z_][\w.\[\]]*\s*[-+*/]?=[^=].*;",
    r"|((ref|static|const|protected|private|autoptr)\s+)*[A-Za-z_][\w<>,]*\s+[A-Za-z_]\w*\s*(=.*)?;)$",
);

/// A path with a line number, such as `TBD_Foo.c:42` or `main.rs#L7`.
const FILE_LINE_REFERENCE: &str =
    r"\b[\w./-]+\.(c|rs|ts|tsx|js|json|md|py|et|layout|conf|toml)(:|#L)[0-9]+\b";

/// Reports every banned term, separator, commented-out statement and file:line reference in the
/// comments of `script`.
///
/// Returns the findings; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let mut patterns: Vec<(Regex, String)> = BANNED_IDENTIFIER_PATTERNS
        .iter()
        .map(|(pattern, what)| {
            (
                Regex::new(pattern).expect("identifier pattern"),
                (*what).to_string(),
            )
        })
        .collect();
    for word in BANNED_WORDS {
        let phrase = word
            .split(' ')
            .map(regex::escape)
            .collect::<Vec<_>>()
            .join(r"\s+");
        let pattern = Regex::new(&format!(r"(?i)\b{phrase}\b")).expect("word pattern");
        patterns.push((pattern, format!("`{word}`")));
    }
    let separator = Regex::new(SEPARATOR).expect("separator pattern");
    let reference = Regex::new(FILE_LINE_REFERENCE).expect("file line pattern");
    let dead_code = Regex::new(COMMENTED_OUT_CODE).expect("commented-out code pattern");
    let mut findings = Vec::new();
    for (index, line) in script.lines.iter().enumerate() {
        for comment in &line.comments {
            let text = comment.text.as_str();
            for (pattern, what) in &patterns {
                if let Some(found) = pattern.find(text) {
                    findings.push(Finding::new(
                        RuleId::ContextFreeProse,
                        index + 1,
                        format!("comment carries {what} (`{}`)", found.as_str()),
                    ));
                }
            }
            if separator.is_match(text) {
                findings.push(Finding::new(
                    RuleId::ContextFreeProse,
                    index + 1,
                    "separator or banner line",
                ));
            }
            if comment.marker != CommentMarker::Block && dead_code.is_match(comment.text.trim()) {
                findings.push(Finding::new(
                    RuleId::ContextFreeProse,
                    index + 1,
                    "commented-out code",
                ));
            }
            if let Some(found) = reference.find(text) {
                findings.push(Finding::new(
                    RuleId::ContextFreeProse,
                    index + 1,
                    format!(
                        "file:line reference `{}`; name the symbol instead",
                        found.as_str()
                    ),
                ));
            }
        }
    }
    findings
}
