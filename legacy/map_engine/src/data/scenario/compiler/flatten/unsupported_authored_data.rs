//! Role: the authored gameplay data a compiled document cannot carry.
//! Position: `mission/compiler/flatten` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: every supported authored field reaches the document; what cannot reach it and
//! changes how the mission plays is listed, one actionable line per authored path, so a mission
//! is refused before it runs instead of running as something other than what was authored.
//!
//! Three kinds of authored data change gameplay when the compile cannot carry them:
//! - a warning-severity drop: the squad leader designation, a vehicle roster row, or a win rule
//!   the document cannot express (labels the compile drops are informational and not listed);
//! - a kit substitution: a character with no `kit-aliases.json` row spawns as its faction's
//!   default instead of the character placed;
//! - authored editor triggers: `mission.schema.json` declares `editorTriggers`, and the compile
//!   does not emit them, so an authored trigger would never fire.

use super::ModMissionDocument;
use crate::data::scenario::validate::Severity;

/// Every authored path the compiled document cannot carry that changes gameplay, with what the
/// author must change. Empty when the document plays exactly as authored.
#[must_use]
pub fn unsupported_authored_data(
    document: &ModMissionDocument,
    payload: &serde_json::Value,
) -> Vec<String> {
    let mut unsupported: Vec<String> = document
        .diagnostics
        .iter()
        .filter(|finding| finding.severity == Severity::Warning)
        .map(|finding| format!("{}: {}", finding.subject, finding.message))
        .collect();
    unsupported.extend(
        document
            .kit_substitutions
            .details()
            .into_iter()
            .map(|line| format!("/editor/slots: {line}")),
    );
    if let Some(triggers) = payload
        .pointer("/editor/triggersById")
        .and_then(serde_json::Value::as_object)
    {
        unsupported.extend(triggers.keys().map(|id| {
            format!(
                "/editor/triggersById/{id}: editor triggers are not carried by the mission \
                 document, so the game would never fire this trigger; remove it before submitting"
            )
        }));
    }
    unsupported
}

#[cfg(test)]
#[path = "tests/unsupported_authored_data.rs"]
mod tests;
