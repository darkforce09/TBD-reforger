//! ECM-6: cross-boundary tags: `@route` on REST call sites, `@contract` on schema-backed DTOs.
//!
//! **Role:** requires `//! @route <METHOD> <path>` in the banner of every method whose body calls
//! `TBD_GameRuntimeHttp.Post`/`Get` or uses `RestContext`; `//! @contract <schema>#<pointer>` in
//! the banner of every class named `*Struct`; that a class carrying `@contract` is named `*Struct`;
//! and that a class deriving from `JsonApiStruct` is named `*Struct` or `*Wire`.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:** detection reads body code with comments and string contents removed, so a
//! call written in a comment or a string never demands a tag; modded classes are never judged by
//! name.

use regex::Regex;

use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};
use super::script_outline::ItemKind;

/// Body-code patterns of a REST call site.
const REST_CALL: &str = r"\bTBD_GameRuntimeHttp\s*\.\s*(Post|Get)\s*\(|\bRestContext\b";

/// A well-formed `@route` tag.
const ROUTE_TAG: &str = r"^@route\s+(GET|POST|PUT|PATCH|DELETE)\s+/\S*";

/// A well-formed `@contract` tag.
const CONTRACT_TAG: &str = r"^@contract\s+[\w./-]+#\S*";

/// Reports the missing and misplaced boundary tags of `script`.
///
/// Returns the findings; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let rest_call = Regex::new(REST_CALL).expect("rest call pattern");
    let route = Regex::new(ROUTE_TAG).expect("route tag pattern");
    let contract = Regex::new(CONTRACT_TAG).expect("contract tag pattern");
    let mut findings = Vec::new();
    for item in &script.outline.items {
        let banner = script.item_banner(item);
        match &item.kind {
            ItemKind::Method
                if rest_call.is_match(&item.body_code)
                    && !banner.iter().any(|(_, text)| route.is_match(text)) =>
            {
                findings.push(Finding::new(
                    RuleId::BoundaryTags,
                    item.start_line,
                    format!(
                        "method `{}` makes a REST call but its banner has no //! @route <METHOD> <path>",
                        item.name
                    ),
                ));
            }
            ItemKind::Class {
                modded: false,
                base,
            } => {
                let tagged = banner.iter().any(|(_, text)| contract.is_match(text));
                let name = item.name.as_str();
                if name.ends_with("Struct") && !tagged {
                    findings.push(Finding::new(
                        RuleId::BoundaryTags,
                        item.start_line,
                        format!("class `{name}` is named *Struct but its banner has no //! @contract <schema>#<pointer>"),
                    ));
                }
                if tagged && !name.ends_with("Struct") {
                    findings.push(Finding::new(
                        RuleId::BoundaryTags,
                        item.start_line,
                        format!("class `{name}` carries @contract, so it is schema-backed and is named *Struct"),
                    ));
                }
                let derives_json = base.as_deref() == Some("JsonApiStruct");
                if derives_json && !name.ends_with("Struct") && !name.ends_with("Wire") {
                    findings.push(Finding::new(
                        RuleId::BoundaryTags,
                        item.start_line,
                        format!("class `{name}` derives JsonApiStruct: name it *Struct (schema-backed) or *Wire (internal)"),
                    ));
                }
            }
            _ => {}
        }
    }
    findings
}
