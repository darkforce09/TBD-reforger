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
use map_engine_core::mission::wire_safety::{self, CargoPhysCatalog};
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
    run_parsed(cell, schema_src, raw, bad_json, |_instance| Vec::new())
}

/// Schema pass plus a code-side walk over the SAME parsed instance. Reuses the parse rather than
/// taking `&[u8]`, because re-parsing a save payload (hundreds of MB at editor scale) to run a
/// second check would cost orders of magnitude more than the check.
fn run_parsed(
    cell: &'static OnceLock<Result<Validator, String>>,
    schema_src: &str,
    raw: &[u8],
    bad_json: &str,
    extra: impl FnOnce(&Value) -> Vec<String>,
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
/// Two code-side passes after the schema, one parse:
/// * [`wire_safety::scan_editor_payload`] (T-181.44) — control characters in authored strings;
/// * [`wire_safety::scan_cargo_capacity`] (T-416) — over-capacity cargo when a phys catalog is
///   supplied via [`validate_mission_editor_payload_with_catalog`]. This entry point passes an
///   **empty** catalog so existing callers keep their signature; without phys attrs the cargo
///   walk is a no-op (never invent capacity). Wire Save/compile refusal by loading
///   `registry_items` into a [`CargoPhysCatalog`] and calling the `_with_catalog` variant.
///
/// @contract mission-editor-payload.schema.json#/ + mission.schema.json#/$defs/wireSafeString
pub fn validate_mission_editor_payload(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    validate_mission_editor_payload_with_catalog(raw, &CargoPhysCatalog::new())
}

/// T-416 — same as [`validate_mission_editor_payload`], but the cargo-capacity walk uses `catalog`
/// (`resource_name →` weight/volume/garment maxima). Build it from `registry_items`; do not put the
/// registry inside `map-engine-core` (see `wire_safety` module header).
pub fn validate_mission_editor_payload_with_catalog(
    raw: &[u8],
    catalog: &CargoPhysCatalog,
) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run_parsed(&V, EDITOR_SCHEMA, raw, "payload is not valid JSON", |instance| {
        let mut d = wire_safety::scan_editor_payload(instance);
        d.extend(wire_safety::scan_cargo_capacity(instance, catalog));
        d
    })
}

/// T-450 — mirrors `TBD_MissionLoader.MISSION_FILE_MAX_BYTES` (`8 * 1024 * 1024`) and
/// `mission.schema.json` `x-tbd-missionFileMaxBytes`. JSON Schema cannot express whole-document
/// byte size on an object, so this code-side check is the enforceable pin that keeps a
/// schema-valid (but oversized) document from reaching the mod and dying at
/// `LoadFromProfileFile`.
const MISSION_FILE_MAX_BYTES: usize = 8 * 1024 * 1024;

/// Validate a compiled mod mission document against `mission.schema.json` (the
/// game-server contract served at `/missions/:id/compiled`).
///
/// @contract mission.schema.json#/
pub fn validate_mission_document(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    if raw.len() > MISSION_FILE_MAX_BYTES {
        return Ok(vec![format!(
            "/: document exceeds MISSION_FILE_MAX_BYTES ({} B > {} B) — \
             TBD_MissionLoader.c LoadFromProfileFile would refuse this file",
            raw.len(),
            MISSION_FILE_MAX_BYTES
        )]);
    }
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
    use map_engine_core::mission::wire_safety::CargoPhys;

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

    /// T-416 — over-capacity cargo joins the same `details` channel as wire-safety, when the
    /// caller supplies a phys catalog. Without a catalog the walk is silent (never invent).
    #[test]
    fn over_capacity_cargo_is_a_save_time_finding_with_catalog() {
        let mut catalog = CargoPhysCatalog::new();
        catalog.insert(
            "mag".into(),
            CargoPhys {
                display_name: "Mag".into(),
                weight_kg: Some(0.5),
                volume_cm3: Some(60.0),
                ..CargoPhys::default()
            },
        );
        catalog.insert(
            "vest_rn".into(),
            CargoPhys {
                display_name: "Plate Carrier".into(),
                max_weight_kg: Some(5.0),
                max_volume_cm3: Some(200.0),
                ..CargoPhys::default()
            },
        );
        // 4 × 60 = 240 > 200 cm³.
        let bad = br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":4}]}}]}}"#;
        let details =
            validate_mission_editor_payload_with_catalog(bad, &catalog).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(
            details[0].starts_with("/editor/slots/0/loadout/wear/vest:"),
            "{details:?}"
        );
        assert!(details[0].contains("240 / 200 cm³"), "{details:?}");

        // One magazine fewer: under capacity → clean.
        let ok = br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":3}]}}]}}"#;
        assert!(
            validate_mission_editor_payload_with_catalog(ok, &catalog)
                .expect("compiles")
                .is_empty(),
            "under-capacity must stay clean"
        );

        // Same over-capacity bytes, empty catalog → silent (signature-compatible entry point).
        assert!(
            validate_mission_editor_payload(bad)
                .expect("compiles")
                .is_empty(),
            "empty catalog must not invent a limit"
        );
    }

    #[test]
    fn mission_document_schema_compiles() {
        // Empty object violates the required keys → non-empty details, but compiles.
        let details = validate_mission_document(b"{}").expect("compiles");
        assert!(!details.is_empty(), "empty doc should be schema-invalid");
    }

    /// T-450 — a document that is schema-shaped can still be refused solely on raw byte size.
    /// Pads `meta.author` (no maxLength) past `MISSION_FILE_MAX_BYTES`; the size finding must
    /// fire before (or instead of) any schema finding, and must name the mod constant.
    #[test]
    fn oversized_mission_document_is_rejected_on_byte_ceiling() {
        let mut doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../packages/tbd-schema/golden-missions/last-stand-at-montfort.json"
        ))
        .expect("golden");
        // Keep the document structurally valid so a missing size check would pass schema alone.
        let pad = "x".repeat(MISSION_FILE_MAX_BYTES);
        doc["meta"]["author"] = serde_json::Value::String(pad);
        let raw = serde_json::to_vec(&doc).expect("serialize");
        assert!(
            raw.len() > MISSION_FILE_MAX_BYTES,
            "pad must push past the ceiling (got {} B)",
            raw.len()
        );

        let details = validate_mission_document(&raw).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(
            details[0].starts_with("/: document exceeds MISSION_FILE_MAX_BYTES"),
            "{details:?}"
        );
        assert!(
            details[0].contains(&MISSION_FILE_MAX_BYTES.to_string()),
            "{details:?}"
        );
    }

    #[test]
    fn schema_x_tbd_mission_file_max_bytes_matches_mod_constant() {
        let schema: serde_json::Value =
            serde_json::from_str(MISSION_SCHEMA).expect("mission.schema.json");
        let pinned = schema["x-tbd-missionFileMaxBytes"]
            .as_u64()
            .expect("x-tbd-missionFileMaxBytes must be present on mission.schema.json");
        assert_eq!(
            pinned as usize, MISSION_FILE_MAX_BYTES,
            "schema keyword drifted from validate_mission_document / TBD_MissionLoader"
        );
    }
}
