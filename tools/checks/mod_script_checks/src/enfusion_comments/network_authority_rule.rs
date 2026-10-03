//! ECM-5: network authority tags on methods, RPCs and replicated properties.
//!
//! **Role:** requires `//! @authority server|client|owner` in the banner of every method whose
//! body calls `Rpc(` or asks where it runs, `//! @rpc <Reliable|Unreliable> <Server|Owner|Broadcast>`
//! directly above every `[RplRpc]` and matching it, and `//! @replicated <prop>` directly above
//! every `[RplProp]`.
//!
//! **Position:** called by [`super::check_script`] on a [`CheckedScript`]; its findings join the
//! report of `cargo xtask verify enfusion-comments`.
//!
//! **Signals & state:** none; pure function.
//!
//! **Invariants:**
//! - A method depends on where it runs when its body code (comments and string contents removed)
//!   holds one of [`LOCATION_SIGNALS`]; the list is closed, so the rule is deterministic.
//! - "Directly above" an attribute skips other attribute-only lines, never anything else.

use regex::Regex;

use super::checked_script::CheckedScript;
use super::findings::{Finding, RuleId};
use super::script_outline::{ItemKind, ScriptAttribute};

/// Body-code patterns that make a method depend on the machine it runs on.
///
/// `TBD_Authority.IsClient()` and `TBD_Authority.IsServer()` are the mod's wrappers over
/// `RplSession.Mode()`, so a call to either asks where the method runs.
const LOCATION_SIGNALS: &str = r"\bRpc\s*\(|\bReplication\s*\.\s*(IsServer|IsClient|IsRunning)\s*\(|\bRplSession\s*\.\s*Mode\s*\(|\bTBD_Authority\s*\.\s*(IsClient|IsServer)\s*\(";

/// Reports the missing and mismatched network tags of `script`.
///
/// Returns the findings; never fails.
pub(crate) fn check(script: &CheckedScript) -> Vec<Finding> {
    let signals = Regex::new(LOCATION_SIGNALS).expect("location signal pattern");
    let authority = Regex::new(r"^@authority\s+(\S+)").expect("authority pattern");
    let mut findings = Vec::new();
    for item in &script.outline.items {
        for attribute in &item.attributes {
            match attribute.name.as_str() {
                "RplRpc" => check_rpc(script, attribute, &mut findings),
                "RplProp" => check_replicated(script, attribute, &mut findings),
                _ => {}
            }
        }
        if item.kind != ItemKind::Method {
            continue;
        }
        let banner = script.item_banner(item);
        let tags: Vec<(usize, &str)> = banner
            .iter()
            .filter_map(|(line, text)| {
                authority
                    .captures(text)
                    .map(|c| (*line, c.get(1).map_or("", |m| m.as_str())))
            })
            .collect();
        for (line, value) in &tags {
            if !matches!(*value, "server" | "client" | "owner") {
                findings.push(Finding::new(
                    RuleId::NetworkAuthority,
                    *line,
                    format!("@authority `{value}` is not server, client or owner"),
                ));
            }
        }
        if tags.is_empty() && signals.is_match(&item.body_code) {
            findings.push(Finding::new(
                RuleId::NetworkAuthority,
                item.start_line,
                format!(
                    "method `{}` calls Rpc( or asks where it runs but its banner has no //! @authority",
                    item.name
                ),
            ));
        }
    }
    findings
}

/// Requires `//! @rpc <channel> <receiver>` directly above an `[RplRpc]`, matching its arguments.
fn check_rpc(script: &CheckedScript, attribute: &ScriptAttribute, findings: &mut Vec<Finding>) {
    let expected = format!(
        "{} {}",
        argument(&attribute.text, r"RplChannel\s*\.\s*(Reliable|Unreliable)").unwrap_or("?"),
        argument(&attribute.text, r"RplRcver\s*\.\s*(Server|Owner|Broadcast)").unwrap_or("?"),
    );
    let tag = script
        .tag_above_attribute(attribute)
        .and_then(|text| text.strip_prefix("@rpc"));
    let Some(tag) = tag else {
        findings.push(Finding::new(
            RuleId::NetworkAuthority,
            attribute.start_line,
            format!("[RplRpc] has no //! @rpc {expected} directly above it"),
        ));
        return;
    };
    let written = tag.split_whitespace().collect::<Vec<_>>().join(" ");
    if written != expected {
        findings.push(Finding::new(
            RuleId::NetworkAuthority,
            attribute.start_line,
            format!("//! @rpc says `{written}`; the [RplRpc] below it is `{expected}`"),
        ));
    }
}

/// Requires `//! @replicated <prop>` directly above an `[RplProp]`.
fn check_replicated(
    script: &CheckedScript,
    attribute: &ScriptAttribute,
    findings: &mut Vec<Finding>,
) {
    let named = script
        .tag_above_attribute(attribute)
        .and_then(|text| text.strip_prefix("@replicated"))
        .is_some_and(|rest| rest.starts_with(char::is_whitespace) && !rest.trim().is_empty());
    if !named {
        findings.push(Finding::new(
            RuleId::NetworkAuthority,
            attribute.start_line,
            "[RplProp] has no //! @replicated <prop> directly above it",
        ));
    }
}

/// The first capture group of `pattern` in `text`.
fn argument<'a>(text: &'a str, pattern: &str) -> Option<&'a str> {
    Regex::new(pattern)
        .expect("attribute argument pattern")
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|m| m.as_str())
}
