//! ECM-3: a `//!` banner sits directly above every class, modded class, enum and method.
//!
//! **Role:** reports every type or method declaration whose first line above, attribute-only
//! lines skipped, is not a `//!` banner line.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:** the finding sits on the declaration's first line; a banner separated from its
//! declaration by a blank line, a separator or a plain `//` comment does not count.

use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};
use super::script_outline::ItemKind;

/// Reports every class, enum and method of `script` without a banner.
///
/// Returns one [`Finding`] per undocumented declaration; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let mut findings = Vec::new();
    for item in &script.outline.items {
        let what = match &item.kind {
            ItemKind::Class { modded: true, .. } => "modded class",
            ItemKind::Class { modded: false, .. } => "class",
            ItemKind::Enum => "enum",
            ItemKind::Method => "method",
            ItemKind::Field | ItemKind::EnumMember => continue,
        };
        if script.item_banner(item).is_empty() {
            findings.push(Finding::new(
                RuleId::DeclarationBanner,
                item.start_line,
                format!("{what} `{}` has no //! banner directly above it", item.name),
            ));
        }
    }
    findings
}
