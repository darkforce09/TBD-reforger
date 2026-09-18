// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-editor-payload.schema.json — regenerate with: cargo xtask ci schema-codegen

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
///One authored faction row. `key` here is deliberately not `mission.schema.json#/$defs/factionKey`. That `$def` is `^[a-z][a-z0-9_]*$`, it is correct, and it belongs to the other layer: it describes the compiled game-server document, where the compiler slugs the key on the way through and keys `orbat`, `slots[].faction` and `briefings` identically. This schema describes the author's raw graph, whose faction-key vocabulary is uppercase by construction — the four canonical sides are BLUFOR, OPFOR, INDFOR and CIV, the editor sets `key = side`, and `faction-library.schema.json` pins the same four as an enum — so the lowercase pattern would reject every faction the live Mission Creator can produce. The vocabulary is not closed at this layer either, since the API accepts keys such as `USA` that appear in no enum, so an enum would be wrong here too. The ORBAT slot table's own `faction` column is a third thing that no schema describes, correctly, because it is a database column rather than a wire document: the ORBAT derivation copies this field verbatim into the squad template, which is why that table holds uppercase keys — this layer's vocabulary arriving unchanged, as designed. What is constrained here is `required: [key]` plus `minLength: 1`, and nothing else. An absent or empty key is not a vocabulary question: the squad template's `faction` field is `#[serde(default)]`, so an empty key lands an empty `orbat_slots.faction`, matches no armory group, and renders an Event Hub dossier card with zero items. Rejecting it here kills that at the write boundary, before it can reach the table. This rejects; it does not transform. Both sides of that join store bytes verbatim, so a one-sided trim here would break the case where an ORBAT value and an armory value agree with identical padding. Whitespace padding is therefore deliberately not constrained — that rule belongs in Rust at one site, because ECMA-262's `\s` is a strict superset of Rust's `char::is_whitespace` (U+FEFF), and one rule expressed in two languages whose definitions differ is the disagreement this design avoids. Also not covered: the explicit top-level `orbat[]`, the other input to the ORBAT template parser, which wins when present and is absent from every live payload. `additionalProperties` is deliberately left open: the row is the wire for authored per-faction briefing prose (`briefing.{situation,mission,execution,markers}`) and the document hydrate path reloads every non-`id` field verbatim, so closing it would make the graph lossy on reload and break the next slice that adds a field before this schema hears about it.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "One authored faction row. `key` here is deliberately not `mission.schema.json#/$defs/factionKey`. That `$def` is `^[a-z][a-z0-9_]*$`, it is correct, and it belongs to the other layer: it describes the compiled game-server document, where the compiler slugs the key on the way through and keys `orbat`, `slots[].faction` and `briefings` identically. This schema describes the author's raw graph, whose faction-key vocabulary is uppercase by construction — the four canonical sides are BLUFOR, OPFOR, INDFOR and CIV, the editor sets `key = side`, and `faction-library.schema.json` pins the same four as an enum — so the lowercase pattern would reject every faction the live Mission Creator can produce. The vocabulary is not closed at this layer either, since the API accepts keys such as `USA` that appear in no enum, so an enum would be wrong here too. The ORBAT slot table's own `faction` column is a third thing that no schema describes, correctly, because it is a database column rather than a wire document: the ORBAT derivation copies this field verbatim into the squad template, which is why that table holds uppercase keys — this layer's vocabulary arriving unchanged, as designed. What is constrained here is `required: [key]` plus `minLength: 1`, and nothing else. An absent or empty key is not a vocabulary question: the squad template's `faction` field is `#[serde(default)]`, so an empty key lands an empty `orbat_slots.faction`, matches no armory group, and renders an Event Hub dossier card with zero items. Rejecting it here kills that at the write boundary, before it can reach the table. This rejects; it does not transform. Both sides of that join store bytes verbatim, so a one-sided trim here would break the case where an ORBAT value and an armory value agree with identical padding. Whitespace padding is therefore deliberately not constrained — that rule belongs in Rust at one site, because ECMA-262's `\\s` is a strict superset of Rust's `char::is_whitespace` (U+FEFF), and one rule expressed in two languages whose definitions differ is the disagreement this design avoids. Also not covered: the explicit top-level `orbat[]`, the other input to the ORBAT template parser, which wins when present and is absent from every live payload. `additionalProperties` is deliberately left open: the row is the wire for authored per-faction briefing prose (`briefing.{situation,mission,execution,markers}`) and the document hydrate path reloads every non-`id` field verbatim, so closing it would make the graph lossy on reload and break the next slice that adds a field before this schema hears about it.",
///  "type": "object",
///  "required": [
///    "key"
///  ],
///  "properties": {
///    "key": {
///      "description": "The author's faction key, stored and read VERBATIM. Required and non-empty; deliberately NOT matched against `mission.schema.json#/$defs/factionKey` — see this row's note for the layer argument and the measurements.",
///      "type": "string",
///      "minLength": 1
///    }
///  }
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
pub struct EditorFaction {
    ///The author's faction key, stored and read VERBATIM. Required and non-empty; deliberately NOT matched against `mission.schema.json#/$defs/factionKey` — see this row's note for the layer argument and the measurements.
    pub key: EditorFactionKey,
}
///The author's faction key, stored and read VERBATIM. Required and non-empty; deliberately NOT matched against `mission.schema.json#/$defs/factionKey` — see this row's note for the layer argument and the measurements.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "The author's faction key, stored and read VERBATIM. Required and non-empty; deliberately NOT matched against `mission.schema.json#/$defs/factionKey` — see this row's note for the layer argument and the measurements.",
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct EditorFactionKey(::std::string::String);
impl ::std::ops::Deref for EditorFactionKey {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<EditorFactionKey> for ::std::string::String {
    fn from(value: EditorFactionKey) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for EditorFactionKey {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for EditorFactionKey {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EditorFactionKey {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EditorFactionKey {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for EditorFactionKey {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///One authored editor layer row.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "One authored editor layer row.",
///  "type": "object",
///  "required": [
///    "id"
///  ],
///  "properties": {
///    "collapsed": {
///      "type": "boolean"
///    },
///    "color": {
///      "type": "string"
///    },
///    "entityIds": {
///      "type": "array",
///      "items": {
///        "type": "string"
///      }
///    },
///    "hidden": {
///      "type": "boolean"
///    },
///    "id": {
///      "type": "string"
///    },
///    "locked": {
///      "type": "boolean"
///    },
///    "name": {
///      "type": "string"
///    },
///    "parentId": {
///      "type": [
///        "string",
///        "null"
///      ]
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EditorLayer {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub collapsed: ::std::option::Option<bool>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub color: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "entityIds",
        default,
        skip_serializing_if = "::std::vec::Vec::is_empty"
    )]
    pub entity_ids: ::std::vec::Vec<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub hidden: ::std::option::Option<bool>,
    pub id: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub locked: ::std::option::Option<bool>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub name: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "parentId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub parent_id: ::std::option::Option<::std::string::String>,
}
///One authored slot row.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "One authored slot row.",
///  "type": "object",
///  "required": [
///    "id"
///  ],
///  "properties": {
///    "assetId": {
///      "type": "string"
///    },
///    "callsign": {
///      "type": "string"
///    },
///    "description": {
///      "type": "string"
///    },
///    "editorHidden": {
///      "type": "boolean"
///    },
///    "id": {
///      "type": "string"
///    },
///    "index": {
///      "type": "integer"
///    },
///    "loadout": {
///      "type": [
///        "object",
///        "string",
///        "null"
///      ]
///    },
///    "loadoutId": {
///      "type": [
///        "string",
///        "null"
///      ]
///    },
///    "position": {
///      "type": "object",
///      "properties": {
///        "rotation": {
///          "type": "number"
///        },
///        "x": {
///          "type": "number"
///        },
///        "y": {
///          "type": "number"
///        },
///        "z": {
///          "type": "number"
///        }
///      },
///      "additionalProperties": false
///    },
///    "rank": {
///      "type": "string"
///    },
///    "role": {
///      "type": "string"
///    },
///    "squadId": {
///      "type": "string"
///    },
///    "stance": {
///      "type": "string"
///    },
///    "tag": {
///      "type": "string"
///    },
///    "unitName": {
///      "type": "string"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EditorSlot {
    #[serde(
        rename = "assetId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub asset_id: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub callsign: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub description: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "editorHidden",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub editor_hidden: ::std::option::Option<bool>,
    pub id: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub index: ::std::option::Option<i64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub loadout: ::std::option::Option<EditorSlotLoadout>,
    #[serde(
        rename = "loadoutId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub loadout_id: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub position: ::std::option::Option<EditorSlotPosition>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub rank: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub role: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "squadId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub squad_id: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub stance: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub tag: ::std::option::Option<::std::string::String>,
    #[serde(
        rename = "unitName",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub unit_name: ::std::option::Option<::std::string::String>,
}
///`EditorSlotLoadout`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": [
///    "object",
///    "string",
///    "null"
///  ]
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum EditorSlotLoadout {
    Null,
    String(::std::string::String),
    Object(::serde_json::Map<::std::string::String, ::serde_json::Value>),
}
impl ::std::convert::From<::serde_json::Map<::std::string::String, ::serde_json::Value>>
    for EditorSlotLoadout
{
    fn from(value: ::serde_json::Map<::std::string::String, ::serde_json::Value>) -> Self {
        Self::Object(value)
    }
}
///`EditorSlotPosition`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "properties": {
///    "rotation": {
///      "type": "number"
///    },
///    "x": {
///      "type": "number"
///    },
///    "y": {
///      "type": "number"
///    },
///    "z": {
///      "type": "number"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EditorSlotPosition {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub rotation: ::std::option::Option<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub x: ::std::option::Option<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub y: ::std::option::Option<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub z: ::std::option::Option<f64>,
}
impl ::std::default::Default for EditorSlotPosition {
    fn default() -> Self {
        Self {
            rotation: Default::default(),
            x: Default::default(),
            y: Default::default(),
            z: Default::default(),
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
///      "description": "Lossless editor graph. `squads` is intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots; the rules those arrays do need are expressed in CODE instead — `crates/map-engine-core/src/mission/wire_safety.rs`. `slots` and `editorLayers` carry per-item subschemas constraining known fields with additionalProperties: false.",
///      "type": "object",
///      "properties": {
///        "editorLayers": {
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/editorLayer"
///          }
///        },
///        "factions": {
///          "description": "Faction rows, carried VERBATIM from the document core (`compile_payload` clones `factionsById` whole). Bounded in practice: the editor mints at most one row per side (`ensure_side_faction` → `add_faction(faction-{SIDE}, key=SIDE, name=SIDE)`), gated on three literals, and the largest live payload holds ONE. So `items` here costs O(factions) ≤ 4, not O(slots) — which is why this array carries a subschema and its three siblings deliberately do not.",
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/editorFaction"
///          }
///        },
///        "slots": {
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/editorSlot"
///          }
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
///    },
///    "winConditions": {
///      "description": "The authored win rule, and the first member of the authored-block passthrough. The map engine's scenario extensions module holds `AUTHORED_BLOCKS`: one row per top-level block the editor authors and the compile carries verbatim, each with a typed validator. The compile copies every listed key from the document's `meta` bag onto this payload root in one generic loop, so a new authored block adds one row plus one validator and leaves the copy loop alone. A block reaches this root only if its row exists — an unlisted key stays parked in `payloadExtras` — so the passthrough is a list, not an open door. A row alone is not enough to reach `/compiled`: the flatten side reads blocks back through a named-field deserialise struct with no `#[serde(flatten)]` catch-all (a flatten catch-all makes serde buffer every unmatched top-level key of an 8 MB payload instead of skipping it) and through a hand `match` ending `_ => None`, so a listed block with no named field and no match arm is dropped in silence: the compile succeeds, no diagnostic is raised, and `/compiled` simply does not contain what the author wrote. A new block therefore moves in two places — the `AUTHORED_BLOCKS` row and the named field plus match arm on the flatten side — and the `every_authored_block_key_reaches_the_wire` test walks `AUTHORED_BLOCKS` and fails by name on a row missing either half. The root stays open for the reason `$defs/editorFaction` gives: declaring `winConditions` here makes it a known part of this contract without closing the root, and closing the root would make the graph lossy on reload and break the next slice that adds a key before this schema hears about it. `mode` carries five values here and seven in `mission.schema.json` because these are the five the editor can author; the compiled contract also carries `points_then_attrition` and `defender_holds_or_attacker_destroys`, hand-authored golden values no editor payload can produce. Narrowing here is free: it rejects nothing the editor can emit and it catches a hand-staged payload that invents a sixth rule at the write boundary rather than at `/compiled`.",
///      "type": "object",
///      "required": [
///        "endOn",
///        "mode"
///      ],
///      "properties": {
///        "endOn": {
///          "type": "array",
///          "items": {
///            "type": "string",
///            "enum": [
///              "time_limit",
///              "all_objectives_captured",
///              "faction_eliminated",
///              "objective_destroyed",
///              "hold_expired"
///            ]
///          },
///          "minItems": 1
///        },
///        "extractionZoneId": {
///          "type": "string",
///          "minLength": 1
///        },
///        "mode": {
///          "type": "string",
///          "enum": [
///            "attrition",
///            "objective",
///            "extraction",
///            "vip",
///            "timeout"
///          ]
///        },
///        "timeoutMinutes": {
///          "type": "integer",
///          "maximum": 1440.0,
///          "minimum": 1.0
///        },
///        "vipSlotId": {
///          "type": "string",
///          "minLength": 1
///        }
///      }
///    },
///    "zones": {
///      "description": "Authored play-area and objective zones, carried into the compiled document by the compiler's derive_zones step. The key is declared so it is a known part of this contract rather than something the open root tolerates. There is no per-item subschema here: the zone rule vocabulary is closed by mission.schema.json#/$defs/zoneRules, and the API enforces it at save (apps/website/api_v2/src/missions/contract/zone_quantisation.rs, scan_authored_zones) against the post-quantisation row that a schema on this document cannot see. The compiler rounds every zone coordinate to 0.1 m and tests the radius before rounding, so a radius that satisfies exclusiveMinimum 0 on the way in can violate it on the way out; the save-time scan applies the same rounding first. Two keys stay unchecked at save on purpose: faction is uppercase at this layer and slugged by the compiler into the compiled factionKey, and a zone the compile drops (no usable shape, empty id, empty type) never reaches a game server.",
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
    #[serde(
        rename = "winConditions",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub win_conditions: ::std::option::Option<TbdMissionEditorPayloadWinConditions>,
    ///Authored play-area and objective zones, carried into the compiled document by the compiler's derive_zones step. The key is declared so it is a known part of this contract rather than something the open root tolerates. There is no per-item subschema here: the zone rule vocabulary is closed by mission.schema.json#/$defs/zoneRules, and the API enforces it at save (apps/website/api_v2/src/missions/contract/zone_quantisation.rs, scan_authored_zones) against the post-quantisation row that a schema on this document cannot see. The compiler rounds every zone coordinate to 0.1 m and tests the radius before rounding, so a radius that satisfies exclusiveMinimum 0 on the way in can violate it on the way out; the save-time scan applies the same rounding first. Two keys stay unchecked at save on purpose: faction is uppercase at this layer and slugged by the compiler into the compiled factionKey, and a zone the compile drops (no usable shape, empty id, empty type) never reaches a game server.
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub zones: ::std::vec::Vec<::serde_json::Value>,
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
            win_conditions: Default::default(),
            zones: Default::default(),
        }
    }
}
///Lossless editor graph. `squads` is intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots; the rules those arrays do need are expressed in CODE instead — `crates/map-engine-core/src/mission/wire_safety.rs`. `slots` and `editorLayers` carry per-item subschemas constraining known fields with additionalProperties: false.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Lossless editor graph. `squads` is intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots; the rules those arrays do need are expressed in CODE instead — `crates/map-engine-core/src/mission/wire_safety.rs`. `slots` and `editorLayers` carry per-item subschemas constraining known fields with additionalProperties: false.",
///  "type": "object",
///  "properties": {
///    "editorLayers": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/editorLayer"
///      }
///    },
///    "factions": {
///      "description": "Faction rows, carried VERBATIM from the document core (`compile_payload` clones `factionsById` whole). Bounded in practice: the editor mints at most one row per side (`ensure_side_faction` → `add_faction(faction-{SIDE}, key=SIDE, name=SIDE)`), gated on three literals, and the largest live payload holds ONE. So `items` here costs O(factions) ≤ 4, not O(slots) — which is why this array carries a subschema and its three siblings deliberately do not.",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/editorFaction"
///      }
///    },
///    "slots": {
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/editorSlot"
///      }
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
    pub editor_layers: ::std::vec::Vec<EditorLayer>,
    ///Faction rows, carried VERBATIM from the document core (`compile_payload` clones `factionsById` whole). Bounded in practice: the editor mints at most one row per side (`ensure_side_faction` → `add_faction(faction-{SIDE}, key=SIDE, name=SIDE)`), gated on three literals, and the largest live payload holds ONE. So `items` here costs O(factions) ≤ 4, not O(slots) — which is why this array carries a subschema and its three siblings deliberately do not.
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub factions: ::std::vec::Vec<EditorFaction>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub slots: ::std::vec::Vec<EditorSlot>,
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
///The authored win rule, and the first member of the authored-block passthrough. The map engine's scenario extensions module holds `AUTHORED_BLOCKS`: one row per top-level block the editor authors and the compile carries verbatim, each with a typed validator. The compile copies every listed key from the document's `meta` bag onto this payload root in one generic loop, so a new authored block adds one row plus one validator and leaves the copy loop alone. A block reaches this root only if its row exists — an unlisted key stays parked in `payloadExtras` — so the passthrough is a list, not an open door. A row alone is not enough to reach `/compiled`: the flatten side reads blocks back through a named-field deserialise struct with no `#[serde(flatten)]` catch-all (a flatten catch-all makes serde buffer every unmatched top-level key of an 8 MB payload instead of skipping it) and through a hand `match` ending `_ => None`, so a listed block with no named field and no match arm is dropped in silence: the compile succeeds, no diagnostic is raised, and `/compiled` simply does not contain what the author wrote. A new block therefore moves in two places — the `AUTHORED_BLOCKS` row and the named field plus match arm on the flatten side — and the `every_authored_block_key_reaches_the_wire` test walks `AUTHORED_BLOCKS` and fails by name on a row missing either half. The root stays open for the reason `$defs/editorFaction` gives: declaring `winConditions` here makes it a known part of this contract without closing the root, and closing the root would make the graph lossy on reload and break the next slice that adds a key before this schema hears about it. `mode` carries five values here and seven in `mission.schema.json` because these are the five the editor can author; the compiled contract also carries `points_then_attrition` and `defender_holds_or_attacker_destroys`, hand-authored golden values no editor payload can produce. Narrowing here is free: it rejects nothing the editor can emit and it catches a hand-staged payload that invents a sixth rule at the write boundary rather than at `/compiled`.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "The authored win rule, and the first member of the authored-block passthrough. The map engine's scenario extensions module holds `AUTHORED_BLOCKS`: one row per top-level block the editor authors and the compile carries verbatim, each with a typed validator. The compile copies every listed key from the document's `meta` bag onto this payload root in one generic loop, so a new authored block adds one row plus one validator and leaves the copy loop alone. A block reaches this root only if its row exists — an unlisted key stays parked in `payloadExtras` — so the passthrough is a list, not an open door. A row alone is not enough to reach `/compiled`: the flatten side reads blocks back through a named-field deserialise struct with no `#[serde(flatten)]` catch-all (a flatten catch-all makes serde buffer every unmatched top-level key of an 8 MB payload instead of skipping it) and through a hand `match` ending `_ => None`, so a listed block with no named field and no match arm is dropped in silence: the compile succeeds, no diagnostic is raised, and `/compiled` simply does not contain what the author wrote. A new block therefore moves in two places — the `AUTHORED_BLOCKS` row and the named field plus match arm on the flatten side — and the `every_authored_block_key_reaches_the_wire` test walks `AUTHORED_BLOCKS` and fails by name on a row missing either half. The root stays open for the reason `$defs/editorFaction` gives: declaring `winConditions` here makes it a known part of this contract without closing the root, and closing the root would make the graph lossy on reload and break the next slice that adds a key before this schema hears about it. `mode` carries five values here and seven in `mission.schema.json` because these are the five the editor can author; the compiled contract also carries `points_then_attrition` and `defender_holds_or_attacker_destroys`, hand-authored golden values no editor payload can produce. Narrowing here is free: it rejects nothing the editor can emit and it catches a hand-staged payload that invents a sixth rule at the write boundary rather than at `/compiled`.",
///  "type": "object",
///  "required": [
///    "endOn",
///    "mode"
///  ],
///  "properties": {
///    "endOn": {
///      "type": "array",
///      "items": {
///        "type": "string",
///        "enum": [
///          "time_limit",
///          "all_objectives_captured",
///          "faction_eliminated",
///          "objective_destroyed",
///          "hold_expired"
///        ]
///      },
///      "minItems": 1
///    },
///    "extractionZoneId": {
///      "type": "string",
///      "minLength": 1
///    },
///    "mode": {
///      "type": "string",
///      "enum": [
///        "attrition",
///        "objective",
///        "extraction",
///        "vip",
///        "timeout"
///      ]
///    },
///    "timeoutMinutes": {
///      "type": "integer",
///      "maximum": 1440.0,
///      "minimum": 1.0
///    },
///    "vipSlotId": {
///      "type": "string",
///      "minLength": 1
///    }
///  }
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
pub struct TbdMissionEditorPayloadWinConditions {
    #[serde(rename = "endOn")]
    pub end_on: ::std::vec::Vec<TbdMissionEditorPayloadWinConditionsEndOnItem>,
    #[serde(
        rename = "extractionZoneId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub extraction_zone_id:
        ::std::option::Option<TbdMissionEditorPayloadWinConditionsExtractionZoneId>,
    pub mode: TbdMissionEditorPayloadWinConditionsMode,
    #[serde(
        rename = "timeoutMinutes",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub timeout_minutes: ::std::option::Option<::std::num::NonZeroU64>,
    #[serde(
        rename = "vipSlotId",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub vip_slot_id: ::std::option::Option<TbdMissionEditorPayloadWinConditionsVipSlotId>,
}
///`TbdMissionEditorPayloadWinConditionsEndOnItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "enum": [
///    "time_limit",
///    "all_objectives_captured",
///    "faction_eliminated",
///    "objective_destroyed",
///    "hold_expired"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum TbdMissionEditorPayloadWinConditionsEndOnItem {
    #[serde(rename = "time_limit")]
    TimeLimit,
    #[serde(rename = "all_objectives_captured")]
    AllObjectivesCaptured,
    #[serde(rename = "faction_eliminated")]
    FactionEliminated,
    #[serde(rename = "objective_destroyed")]
    ObjectiveDestroyed,
    #[serde(rename = "hold_expired")]
    HoldExpired,
}
impl ::std::fmt::Display for TbdMissionEditorPayloadWinConditionsEndOnItem {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::TimeLimit => f.write_str("time_limit"),
            Self::AllObjectivesCaptured => f.write_str("all_objectives_captured"),
            Self::FactionEliminated => f.write_str("faction_eliminated"),
            Self::ObjectiveDestroyed => f.write_str("objective_destroyed"),
            Self::HoldExpired => f.write_str("hold_expired"),
        }
    }
}
impl ::std::str::FromStr for TbdMissionEditorPayloadWinConditionsEndOnItem {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "time_limit" => Ok(Self::TimeLimit),
            "all_objectives_captured" => Ok(Self::AllObjectivesCaptured),
            "faction_eliminated" => Ok(Self::FactionEliminated),
            "objective_destroyed" => Ok(Self::ObjectiveDestroyed),
            "hold_expired" => Ok(Self::HoldExpired),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for TbdMissionEditorPayloadWinConditionsEndOnItem {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for TbdMissionEditorPayloadWinConditionsEndOnItem
{
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for TbdMissionEditorPayloadWinConditionsEndOnItem
{
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`TbdMissionEditorPayloadWinConditionsExtractionZoneId`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct TbdMissionEditorPayloadWinConditionsExtractionZoneId(::std::string::String);
impl ::std::ops::Deref for TbdMissionEditorPayloadWinConditionsExtractionZoneId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<TbdMissionEditorPayloadWinConditionsExtractionZoneId>
    for ::std::string::String
{
    fn from(value: TbdMissionEditorPayloadWinConditionsExtractionZoneId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for TbdMissionEditorPayloadWinConditionsExtractionZoneId {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for TbdMissionEditorPayloadWinConditionsExtractionZoneId {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for TbdMissionEditorPayloadWinConditionsExtractionZoneId
{
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for TbdMissionEditorPayloadWinConditionsExtractionZoneId
{
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for TbdMissionEditorPayloadWinConditionsExtractionZoneId {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`TbdMissionEditorPayloadWinConditionsMode`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "enum": [
///    "attrition",
///    "objective",
///    "extraction",
///    "vip",
///    "timeout"
///  ]
///}
/// ```
/// </details>
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum TbdMissionEditorPayloadWinConditionsMode {
    #[serde(rename = "attrition")]
    Attrition,
    #[serde(rename = "objective")]
    Objective,
    #[serde(rename = "extraction")]
    Extraction,
    #[serde(rename = "vip")]
    Vip,
    #[serde(rename = "timeout")]
    Timeout,
}
impl ::std::fmt::Display for TbdMissionEditorPayloadWinConditionsMode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Attrition => f.write_str("attrition"),
            Self::Objective => f.write_str("objective"),
            Self::Extraction => f.write_str("extraction"),
            Self::Vip => f.write_str("vip"),
            Self::Timeout => f.write_str("timeout"),
        }
    }
}
impl ::std::str::FromStr for TbdMissionEditorPayloadWinConditionsMode {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "attrition" => Ok(Self::Attrition),
            "objective" => Ok(Self::Objective),
            "extraction" => Ok(Self::Extraction),
            "vip" => Ok(Self::Vip),
            "timeout" => Ok(Self::Timeout),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for TbdMissionEditorPayloadWinConditionsMode {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TbdMissionEditorPayloadWinConditionsMode {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TbdMissionEditorPayloadWinConditionsMode {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`TbdMissionEditorPayloadWinConditionsVipSlotId`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct TbdMissionEditorPayloadWinConditionsVipSlotId(::std::string::String);
impl ::std::ops::Deref for TbdMissionEditorPayloadWinConditionsVipSlotId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<TbdMissionEditorPayloadWinConditionsVipSlotId> for ::std::string::String {
    fn from(value: TbdMissionEditorPayloadWinConditionsVipSlotId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for TbdMissionEditorPayloadWinConditionsVipSlotId {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for TbdMissionEditorPayloadWinConditionsVipSlotId {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for TbdMissionEditorPayloadWinConditionsVipSlotId
{
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for TbdMissionEditorPayloadWinConditionsVipSlotId
{
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for TbdMissionEditorPayloadWinConditionsVipSlotId {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
