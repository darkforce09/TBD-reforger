// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: packages/tbd-schema/schema/mission-editor-payload.schema.json — regenerate with: make schema-codegen

/// Error types.
pub mod error {
    /// Error from a `TryFrom` or `FromStr` implementation.
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
///The 2D-editor 'superset' stored verbatim as a MissionVersion.json_payload (the write side of POST /api/v1/missions/:id/versions; mirrors the frontend compile.ts MissionPayload). This is NOT the canonical mission.schema.json document — that is the game-server contract derived/exported separately. Its integer schemaVersion is the editor-payload format version, a DISTINCT namespace from the canonical mission contract's string schemaVersion. Validation is intentionally lenient on presence (minimal and partial saves are valid, including the empty {} a freshly created mission stores) but strict on type, to reject malformed payloads and the schemaVersion namespace confusion (a string here) before persist.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "$id": "https://schema.tbdevent.eu/mission-editor-payload/v1.json",
///  "title": "TBD Mission Editor Payload",
///  "description": "The 2D-editor 'superset' stored verbatim as a MissionVersion.json_payload (the write side of POST /api/v1/missions/:id/versions; mirrors the frontend compile.ts MissionPayload). This is NOT the canonical mission.schema.json document — that is the game-server contract derived/exported separately. Its integer schemaVersion is the editor-payload format version, a DISTINCT namespace from the canonical mission contract's string schemaVersion. Validation is intentionally lenient on presence (minimal and partial saves are valid, including the empty {} a freshly created mission stores) but strict on type, to reject malformed payloads and the schemaVersion namespace confusion (a string here) before persist.",
///  "type": "object",
///  "properties": {
///    "editor": {
///      "description": "Lossless editor graph. The arrays are intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots.",
///      "type": "object",
///      "properties": {
///        "editorLayers": {
///          "type": "array"
///        },
///        "factions": {
///          "type": "array"
///        },
///        "slots": {
///          "type": "array"
///        },
///        "squads": {
///          "type": "array"
///        }
///      }
///    },
///    "environment": {
///      "type": "object"
///    },
///    "loadouts": {
///      "type": "object"
///    },
///    "map": {
///      "type": "object",
///      "properties": {
///        "bounds": {
///          "type": "array",
///          "items": {
///            "type": "number"
///          }
///        },
///        "terrain": {
///          "type": "string"
///        }
///      }
///    },
///    "markers": {
///      "type": "array"
///    },
///    "objectives": {
///      "type": "array"
///    },
///    "orbat": {
///      "description": "Optional backend ORBAT contract (omitted on Save Version; the server derives it from editor).",
///      "type": "array"
///    },
///    "schemaVersion": {
///      "description": "Editor-payload format version (integer; do not confuse with the canonical mission schemaVersion, which is a string).",
///      "type": "integer"
///    },
///    "vehicles": {
///      "type": "array"
///    }
///  }
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
pub struct TbdMissionEditorPayload {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub editor: ::std::option::Option<TbdMissionEditorPayloadEditor>,
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub environment: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub loadouts: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub map: ::std::option::Option<TbdMissionEditorPayloadMap>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub markers: ::std::vec::Vec<::serde_json::Value>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub objectives: ::std::vec::Vec<::serde_json::Value>,
    ///Optional backend ORBAT contract (omitted on Save Version; the server derives it from editor).
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub orbat: ::std::vec::Vec<::serde_json::Value>,
    ///Editor-payload format version (integer; do not confuse with the canonical mission schemaVersion, which is a string).
    #[serde(
        rename = "schemaVersion",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub schema_version: ::std::option::Option<i64>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub vehicles: ::std::vec::Vec<::serde_json::Value>,
}
impl ::std::default::Default for TbdMissionEditorPayload {
    fn default() -> Self {
        Self {
            editor: Default::default(),
            environment: Default::default(),
            loadouts: Default::default(),
            map: Default::default(),
            markers: Default::default(),
            objectives: Default::default(),
            orbat: Default::default(),
            schema_version: Default::default(),
            vehicles: Default::default(),
        }
    }
}
///Lossless editor graph. The arrays are intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Lossless editor graph. The arrays are intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots.",
///  "type": "object",
///  "properties": {
///    "editorLayers": {
///      "type": "array"
///    },
///    "factions": {
///      "type": "array"
///    },
///    "slots": {
///      "type": "array"
///    },
///    "squads": {
///      "type": "array"
///    }
///  }
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
pub struct TbdMissionEditorPayloadEditor {
    #[serde(
        rename = "editorLayers",
        default,
        skip_serializing_if = "::std::vec::Vec::is_empty"
    )]
    pub editor_layers: ::std::vec::Vec<::serde_json::Value>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub factions: ::std::vec::Vec<::serde_json::Value>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub slots: ::std::vec::Vec<::serde_json::Value>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub squads: ::std::vec::Vec<::serde_json::Value>,
}
impl ::std::default::Default for TbdMissionEditorPayloadEditor {
    fn default() -> Self {
        Self {
            editor_layers: Default::default(),
            factions: Default::default(),
            slots: Default::default(),
            squads: Default::default(),
        }
    }
}
///`TbdMissionEditorPayloadMap`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "properties": {
///    "bounds": {
///      "type": "array",
///      "items": {
///        "type": "number"
///      }
///    },
///    "terrain": {
///      "type": "string"
///    }
///  }
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
pub struct TbdMissionEditorPayloadMap {
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub bounds: ::std::vec::Vec<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub terrain: ::std::option::Option<::std::string::String>,
}
impl ::std::default::Default for TbdMissionEditorPayloadMap {
    fn default() -> Self {
        Self {
            bounds: Default::default(),
            terrain: Default::default(),
        }
    }
}
