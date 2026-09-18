//! Runtime JSON-Schema validation for the mission domain, on the `jsonschema` crate
//! (draft 2020-12).
//!
//! Schemas are embedded directly from the canonical `contracts_v2/definitions/` (no copy step),
//! so the bytes validated here are the bytes the contract declares.
//!
//! The result contract is uniform across every entry point: `Ok(empty)` = valid,
//! `Ok(details)` = schema violations, `Err` = an internal schema-compile failure and nothing else.
//! The `details` strings are worded by the validator crate and are advisory — a caller renders
//! them, never matches on them.

use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::Value;
use website_map_engine::data::scenario::wire_safety::{self, CargoPhysCatalog};

use super::zone_quantisation::scan_authored_zones;

const EDITOR_SCHEMA: &str =
    include_str!("../../../../../../contracts_v2/definitions/mission-editor-payload.schema.json");
/// Shared with [`super::zone_quantisation`], which lifts `#/$defs/zone` out of these same bytes so
/// the save boundary and the serve boundary cannot disagree about the zone vocabulary.
pub(super) const MISSION_SCHEMA: &str =
    include_str!("../../../../../../contracts_v2/definitions/mission.schema.json");
const REGISTRY_ITEMS_SCHEMA: &str =
    include_str!("../../../../../../contracts_v2/definitions/registry-items.schema.json");
const REGISTRY_COMPAT_SCHEMA: &str =
    include_str!("../../../../../../contracts_v2/definitions/registry-compat.schema.json");
const FACTION_LIBRARY_SCHEMA: &str =
    include_str!("../../../../../../contracts_v2/definitions/faction-library.schema.json");

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
/// Three code-side passes after the schema, all on one parse:
/// * [`wire_safety::scan_editor_payload`] — control characters in authored strings;
/// * [`wire_safety::scan_cargo_capacity`] — over-capacity cargo when a phys catalog is supplied
///   via [`validate_mission_editor_payload_with_catalog`]. This entry point passes an **empty**
///   catalog so a caller that has no registry keeps the one-argument signature; without phys
///   attrs the cargo walk is a no-op (never invent capacity). Wire Save/compile refusal by
///   loading `registry_items` into a [`CargoPhysCatalog`] and calling the `_with_catalog` variant;
/// * [`scan_authored_zones`] — the zone vocabulary, measured on the row the compile will emit.
///
/// @contract mission-editor-payload.schema.json#/ + mission.schema.json#/$defs/wireSafeString
pub fn validate_mission_editor_payload(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    validate_mission_editor_payload_with_catalog(raw, &CargoPhysCatalog::new())
}

/// Same as [`validate_mission_editor_payload`], but the cargo-capacity walk uses `catalog`
/// (`resource_name →` weight/volume/garment maxima). Build it from `registry_items`; do not put the
/// registry inside the map engine (see the `wire_safety` module header).
pub fn validate_mission_editor_payload_with_catalog(
    raw: &[u8],
    catalog: &CargoPhysCatalog,
) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run_parsed(
        &V,
        EDITOR_SCHEMA,
        raw,
        "payload is not valid JSON",
        |instance| {
            let mut d = wire_safety::scan_editor_payload(instance);
            d.extend(wire_safety::scan_cargo_capacity(instance, catalog));
            d.extend(scan_authored_zones(instance));
            d
        },
    )
}

/// Mirrors `TBD_MissionLoader.MISSION_FILE_MAX_BYTES` (`8 * 1024 * 1024`) and
/// `mission.schema.json` `x-tbd-missionFileMaxBytes`. JSON Schema cannot express whole-document
/// byte size on an object, so this code-side check is the enforceable pin that keeps a
/// schema-valid (but oversized) document from reaching the mod and dying at
/// `LoadFromProfileFile`.
pub(super) const MISSION_FILE_MAX_BYTES: usize = 8 * 1024 * 1024;

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

/// Validate a raw items envelope against `registry-items.schema.json`
/// (the Workbench export ingested by `import-registry`).
///
/// @contract registry-items.schema.json#/
pub fn validate_registry_items_envelope(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(&V, REGISTRY_ITEMS_SCHEMA, raw, "envelope is not valid JSON")
}

/// Validate a faction-library document against `faction-library.schema.json`
/// (the jsonb doc of a `user_factions` row).
///
/// @contract faction-library.schema.json#/
pub fn validate_faction_library_doc(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(&V, FACTION_LIBRARY_SCHEMA, raw, "doc is not valid JSON")
}

/// Validate a raw compat envelope against `registry-compat.schema.json`
/// (the Workbench edge export ingested by `import-registry`).
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
#[path = "tests/schema_validators.rs"]
mod tests;
