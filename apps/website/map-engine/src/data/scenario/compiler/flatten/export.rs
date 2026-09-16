//! Role: export.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Finding, MissionMeta, apply_authored_environment, flatten_to_mod_document};

/// JSON-in / JSON-out flatten for the client: `meta_json` (camelCase [`MissionMeta`], built from the `GET /missions/:id` row) + the stored version `payload` → the compiled mod-document JSON bytes. Keeps serde_json on this side so the caller stays dependency-thin.
pub fn flatten_mod_document_json(meta_json: &[u8], payload: &[u8]) -> Result<Vec<u8>, String> {
    flatten_mod_document_json_with_substitutions(meta_json, payload).map(|(bytes, _)| bytes)
}

/// Compile once and return both the game-document bytes and the resource-substitution report.
///
/// ```
/// # use website_map_engine::data::scenario::flatten::flatten_mod_document_json_with_substitutions as f;
/// // One slot, carrying a real registry character that has no `kit-aliases.json` row.
/// let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
///   "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
///   "weatherPreset":"clear"}"#;
/// let payload = br#"{"editor":{
///   "factions":[{"key":"BLUFOR","name":"US Army","squadIds":["sq"]}],
///   "squads":[{"id":"sq","callsign":"Alpha","slotIds":["s0"]}],
///   "slots":[{"id":"s0","index":0,"role":"RFL",
///     "assetId":"{0F6689B491641155}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Sniper.et",
///     "position":{"x":1.0,"y":2.0,"z":0.0,"rotation":0.0}}]}}"#;
///
/// let (bytes, substitutions) = f(meta, payload).expect("compiles");
/// let text = String::from_utf8(bytes).expect("utf-8");
/// // The seat really did change: a sniper was placed, a rifleman is what the document ships,
/// // and the document keeps no trace of what was asked for.
/// assert!(text.contains("kit:us_rifleman"), "{text}");
/// assert!(!text.contains("Character_US_Sniper"), "{text}");
/// // …and that is no longer silent.
/// assert_eq!(substitutions.len(), 1, "{substitutions:?}");
/// assert!(substitutions[0].contains("blufor:Alpha:RFL:0"), "{substitutions:?}");
/// ```
///
/// # Errors
/// Returns a message on meta/payload parse failure or a compile error (e.g. no slots).
pub fn flatten_mod_document_json_with_substitutions(
    meta_json: &[u8],
    payload: &[u8],
) -> Result<(Vec<u8>, Vec<String>), String> {
    flatten_mod_document_json_full(meta_json, payload).map(|(bytes, subs, _)| (bytes, subs))
}

/// Flatten mod document json with diagnostics using the supplied domain data.
pub fn flatten_mod_document_json_with_diagnostics(
    meta_json: &[u8],
    payload: &[u8],
) -> Result<(Vec<u8>, Vec<Finding>), String> {
    flatten_mod_document_json_full(meta_json, payload).map(|(bytes, _, findings)| (bytes, findings))
}

/// Domain representation of compiled output.
pub type CompiledOutput = (Vec<u8>, Vec<String>, Vec<Finding>);

/// The one body behind [`flatten_mod_document_json`], [`flatten_mod_document_json_with_substitutions`] and [`flatten_mod_document_json_with_diagnostics`]: bytes + kit substitutions + structured findings, from ONE compile.
pub fn flatten_mod_document_json_full(
    meta_json: &[u8],
    payload: &[u8],
) -> Result<CompiledOutput, String> {
    let mut meta: MissionMeta = serde_json::from_slice(meta_json).map_err(|e| e.to_string())?;
    apply_authored_environment(&mut meta, payload);
    let doc = flatten_to_mod_document(&meta, payload).map_err(|e| e.to_string())?;

    let substitutions = doc.kit_substitutions.details();
    let diagnostics = doc.diagnostics.clone();
    let bytes = serde_json::to_vec(&doc).map_err(|e| e.to_string())?;
    Ok((bytes, substitutions, diagnostics))
}
