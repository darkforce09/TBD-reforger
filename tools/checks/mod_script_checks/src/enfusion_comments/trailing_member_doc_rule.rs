//! ECM-4: a trailing `//!<` documents every field and enum member.
//!
//! **Role:** reports every class-body or top-level field and every enum member none of whose
//! lines carries a `//!<` comment after its code.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:** local variables inside method bodies are never fields; the `//!<` may sit on
//! any line of a multi-line declaration, and the finding sits on its first line.

use super::super::enfusion_script_lexer::CommentMarker;
use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};
use super::script_outline::ItemKind;

/// Reports every field and enum member of `script` without a trailing `//!<`.
///
/// Returns one [`Finding`] per undocumented member; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let mut findings = Vec::new();
    for item in &script.outline.items {
        let what = match item.kind {
            ItemKind::Field => "field",
            ItemKind::EnumMember => "enum member",
            _ => continue,
        };
        let documented = (item.start_line..=item.end_line).any(|number| {
            script.line(number).is_some_and(|line| {
                line.comments
                    .iter()
                    .any(|comment| comment.marker == CommentMarker::TrailingDoc)
            })
        });
        if !documented {
            findings.push(Finding::new(
                RuleId::TrailingMemberDoc,
                item.start_line,
                format!(
                    "{what} `{}` has no trailing //!< doc (unit, default or JSON key)",
                    item.name
                ),
            ));
        }
    }
    findings
}
