//! ECM-7: `[Attribute]` has `desc:` and `[ComponentEditorProps]` has `description:`.
//!
//! **Role:** reports every editor-facing attribute that does not describe itself by name.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:** the named argument is looked for in attribute code with string contents
//! removed, so `desc:` written inside a string literal never satisfies the rule; a positional
//! description does not count.

use regex::Regex;

use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};

/// Attribute names and the named argument each must carry.
const DESCRIBED_ATTRIBUTES: [(&str, &str); 2] = [
    ("Attribute", "desc"),
    ("ComponentEditorProps", "description"),
];

/// Reports every `[Attribute]` without `desc:` and `[ComponentEditorProps]` without `description:`.
///
/// Returns one [`Finding`] per undescribed attribute; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (attribute_name, argument) in DESCRIBED_ATTRIBUTES {
        let named = Regex::new(&format!(r"\b{argument}\s*:")).expect("named argument pattern");
        for item in &script.outline.items {
            for attribute in item.attributes.iter().filter(|a| a.name == attribute_name) {
                if !named.is_match(&attribute.text) {
                    findings.push(Finding::new(
                        RuleId::AttributeDescription,
                        attribute.start_line,
                        format!("[{attribute_name}] on `{}` has no {argument}:", item.name),
                    ));
                }
            }
        }
    }
    findings
}
