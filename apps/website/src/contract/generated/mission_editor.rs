// Code generated from JSON Schema using quicktype. DO NOT EDIT.
// Source: packages/tbd-schema/schema/mission-editor-payload.schema.json — regenerate with: make schema-codegen

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::mission_editor;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: mission_editor = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The 2D-editor 'superset' stored verbatim as a MissionVersion.json_payload (the write side
/// of POST /api/v1/missions/:id/versions; mirrors the frontend compile.ts MissionPayload).
/// This is NOT the canonical mission.schema.json document — that is the game-server contract
/// derived/exported separately. Its integer schemaVersion is the editor-payload format
/// version, a DISTINCT namespace from the canonical mission contract's string schemaVersion.
/// Validation is intentionally lenient on presence (minimal and partial saves are valid,
/// including the empty {} a freshly created mission stores) but strict on type, to reject
/// malformed payloads and the schemaVersion namespace confusion (a string here) before
/// persist.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionEditor {
    /// Lossless editor graph. The arrays are intentionally unconstrained (no per-item schema) so
    /// validation stays O(1) on missions with hundreds of thousands of slots.
    pub editor: Option<Editor>,

    pub environment: Option<HashMap<String, Option<serde_json::Value>>>,

    pub loadouts: Option<HashMap<String, Option<serde_json::Value>>>,

    pub map: Option<Map>,

    pub markers: Option<Vec<Option<serde_json::Value>>>,

    pub objectives: Option<Vec<Option<serde_json::Value>>>,

    /// Optional backend ORBAT contract (omitted on Save Version; the server derives it from
    /// editor).
    pub orbat: Option<Vec<Option<serde_json::Value>>>,

    /// Editor-payload format version (integer; do not confuse with the canonical mission
    /// schemaVersion, which is a string).
    pub schema_version: Option<i64>,

    pub vehicles: Option<Vec<Option<serde_json::Value>>>,
}

/// Lossless editor graph. The arrays are intentionally unconstrained (no per-item schema) so
/// validation stays O(1) on missions with hundreds of thousands of slots.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Editor {
    pub editor_layers: Option<Vec<Option<serde_json::Value>>>,

    pub factions: Option<Vec<Option<serde_json::Value>>>,

    pub slots: Option<Vec<Option<serde_json::Value>>>,

    pub squads: Option<Vec<Option<serde_json::Value>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Map {
    pub bounds: Option<Vec<f64>>,

    pub terrain: Option<String>,
}
