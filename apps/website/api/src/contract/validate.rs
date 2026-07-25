//! Runtime JSON-Schema validation — Rust port of `internal/contract/{validate,mission}.go`,
//! using the `jsonschema` crate (draft 2020-12) in place of santhosh-tekuri.
//!
//! Schemas are embedded directly from the canonical `packages/tbd-schema/schema/`
//! (no copy step). Contract mirrors Go: `Ok(empty)` = valid, `Ok(details)` = schema
//! violations (advisory strings), `Err` = internal schema-compile failure only.
//!
//! NOTE: the `details` *strings* differ from Go's santhosh-tekuri wording (different
//! library) — a documented bounded deviation. Status + top-level error message match;
//! `details` are advisory (never matched by the client).

use std::sync::OnceLock;

use jsonschema::Validator;
use map_engine_core::mission::wire_safety;
use serde_json::Value;

const EDITOR_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/mission-editor-payload.schema.json");
const MISSION_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/mission.schema.json");
const REGISTRY_ITEMS_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/registry-items.schema.json");
const REGISTRY_COMPAT_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/registry-compat.schema.json");
const FACTION_LIBRARY_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/faction-library.schema.json");

/// Internal schema-compile failure (never returned for merely-invalid input).
#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error("schema compile failed: {0}")]
    Compile(String),
}

fn compile(src: &str) -> Result<Validator, String> {
    let schema: Value = serde_json::from_str(src).map_err(|e| e.to_string())?;
    jsonschema::validator_for(&schema).map_err(|e| e.to_string())
}

fn run(
    cell: &'static OnceLock<Result<Validator, String>>,
    schema_src: &str,
    raw: &[u8],
    bad_json: &str,
) -> Result<Vec<String>, ContractError> {
    run_with(cell, schema_src, raw, bad_json, |_| Vec::new())
}

/// [`run`] plus a code-side pass over the SAME parsed instance. `extra` exists for rules that are
/// true of the document but deliberately not expressed in its schema — see
/// [`validate_mission_editor_payload`]. It reuses the parse rather than taking `&[u8]`, because
/// re-parsing a save payload (hundreds of MB at editor scale) to run a second check would cost
/// orders of magnitude more than the check.
fn run_with(
    cell: &'static OnceLock<Result<Validator, String>>,
    schema_src: &str,
    raw: &[u8],
    bad_json: &str,
    extra: fn(&Value) -> Vec<String>,
) -> Result<Vec<String>, ContractError> {
    let compiled = cell.get_or_init(|| compile(schema_src));
    let validator = compiled
        .as_ref()
        .map_err(|e| ContractError::Compile(e.clone()))?;

    let Ok(instance) = serde_json::from_slice::<Value>(raw) else {
        return Ok(vec![bad_json.to_string()]);
    };

    let mut details: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| {
            let loc = e.instance_path().to_string();
            let loc = if loc.is_empty() { "/".to_string() } else { loc };
            format!("{loc}: {e}")
        })
        .collect();
    details.extend(extra(&instance));
    Ok(details)
}

/// Validate a raw mission-version payload against `mission-editor-payload.schema.json`
/// (the write-side editor superset). Used by CreateMission + CreateVersion.
///
/// Two passes, one parse. The schema pass leaves `editor.slots[]` unconstrained on purpose so it
/// stays O(1) on missions with hundreds of thousands of slots; the second pass
/// ([`wire_safety::scan_editor_payload`], T-181.44) covers the one rule that omission lets through —
/// a control character in an authored callsign / role, which compiles into a `wireSafeString` field
/// and is then rejected at `GET /missions/:id/compiled`, hours later, in front of the wrong human.
/// Its findings join the same `details` array, so callers and the wire shape are unchanged.
///
/// @contract mission-editor-payload.schema.json#/ + mission.schema.json#/$defs/wireSafeString
pub fn validate_mission_editor_payload(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run_with(
        &V,
        EDITOR_SCHEMA,
        raw,
        "payload is not valid JSON",
        wire_safety::scan_editor_payload,
    )
}

/// Validate a compiled mod mission document against `mission.schema.json` (the
/// game-server contract served at `/missions/:id/compiled`).
///
/// @contract mission.schema.json#/
pub fn validate_mission_document(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(&V, MISSION_SCHEMA, raw, "document is not valid JSON")
}

/// Validate a raw T-150 items envelope against `registry-items.schema.json`
/// (the Workbench export ingested by `import-registry`, T-068.9).
///
/// @contract registry-items.schema.json#/
pub fn validate_registry_items_envelope(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(&V, REGISTRY_ITEMS_SCHEMA, raw, "envelope is not valid JSON")
}

/// Validate a faction-library document against `faction-library.schema.json`
/// (the jsonb doc of a user_factions row, T-153).
///
/// @contract faction-library.schema.json#/
pub fn validate_faction_library_doc(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(&V, FACTION_LIBRARY_SCHEMA, raw, "doc is not valid JSON")
}

/// Validate a raw T-150 compat envelope against `registry-compat.schema.json`
/// (the Workbench edge export ingested by `import-registry`, T-068.9).
///
/// @contract registry-compat.schema.json#/
pub fn validate_registry_compat_envelope(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(
        &V,
        REGISTRY_COMPAT_SCHEMA,
        raw,
        "envelope is not valid JSON",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_schema_compiles_and_accepts_minimal_payload() {
        // A minimal valid editor payload (schemaVersion int + editor block).
        let ok = br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}"#;
        let details = validate_mission_editor_payload(ok).expect("compiles");
        assert!(details.is_empty(), "expected valid, got {details:?}");
    }

    #[test]
    fn invalid_json_reports_detail_not_error() {
        let details = validate_mission_editor_payload(b"not json").expect("compiles");
        assert_eq!(details, vec!["payload is not valid JSON".to_string()]);
    }

    /// T-181.44 — the save-time catch, on the channel `create_version` already answers 400 with.
    /// Before this, the same payload validated CLEAN here and only failed later at `/compiled`,
    /// as a 500 the author never saw.
    #[test]
    fn control_character_in_an_authored_slot_string_is_a_save_time_finding() {
        let bad = br#"{"schemaVersion":1,"editor":{"factions":[],
            "squads":[{"id":"sq1","callsign":"AL\tPHA","slotIds":["s1"]}],
            "slots":[{"id":"s1","role":"SL"}],"editorLayers":[]}}"#;
        let details = validate_mission_editor_payload(bad).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(
            details[0].starts_with("/editor/squads/0/callsign:"),
            "{details:?}"
        );
        assert!(details[0].contains("TAB (U+0009)"), "{details:?}");

        // The schema half of the same call is untouched: a payload that is merely wire-safe still
        // has to satisfy `mission-editor-payload.schema.json`.
        let ok = br#"{"schemaVersion":1,"editor":{"factions":[],
            "squads":[{"id":"sq1","callsign":"ALPHA","slotIds":["s1"]}],
            "slots":[{"id":"s1","role":"SL"}],"editorLayers":[]}}"#;
        assert!(
            validate_mission_editor_payload(ok)
                .expect("compiles")
                .is_empty()
        );
    }

    #[test]
    fn mission_document_schema_compiles() {
        // Empty object violates the required keys → non-empty details, but compiles.
        let details = validate_mission_document(b"{}").expect("compiles");
        assert!(!details.is_empty(), "empty doc should be schema-invalid");
    }
}
