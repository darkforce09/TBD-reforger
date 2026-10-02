//! ECM-9: the file name is its primary type; one primary type per file.
//!
//! **Role:** requires a top-level class, modded class or enum named exactly as the file stem, and
//! holds every other top-level type to the size of a small companion.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:** a companion spans at most [`COMPANION_MAX_LINES`] lines from its declaration
//! to its closing brace; a larger second type is a second primary type and belongs in its own file.

use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};
use super::script_outline::ItemKind;

/// The most lines a companion type may span, declaration to closing brace.
pub(crate) const COMPANION_MAX_LINES: usize = 60;

/// Checks that `script` is named after its primary type and holds only small companions besides.
///
/// Returns the findings; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let stem = script
        .file_name
        .strip_suffix(".c")
        .unwrap_or(&script.file_name);
    let types: Vec<_> = script
        .outline
        .items
        .iter()
        .filter(|item| {
            item.parent.is_none() && matches!(item.kind, ItemKind::Class { .. } | ItemKind::Enum)
        })
        .collect();
    let mut findings = Vec::new();
    if !types.iter().any(|item| item.name == stem) {
        let declared: Vec<&str> = types.iter().map(|item| item.name.as_str()).collect();
        findings.push(Finding::new(
            RuleId::FileNamesPrimaryType,
            types.first().map_or(1, |item| item.start_line),
            format!(
                "no top-level type is named `{stem}` like the file (declares: {})",
                if declared.is_empty() {
                    "none".to_string()
                } else {
                    declared.join(", ")
                }
            ),
        ));
        return findings;
    }
    for item in types.iter().filter(|item| item.name != stem) {
        let span = item.body_end_line.unwrap_or(item.end_line) + 1 - item.start_line;
        if span > COMPANION_MAX_LINES {
            findings.push(Finding::new(
                RuleId::FileNamesPrimaryType,
                item.start_line,
                format!(
                    "`{}` spans {span} lines, over the {COMPANION_MAX_LINES}-line companion limit; give it its own file",
                    item.name
                ),
            ));
        }
    }
    findings
}
