// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: packages/tbd-schema/schema/mission-editor-payload.schema.json — regenerate with: cargo xtask ci schema-codegen

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
/**One authored faction row.

═══ THE LAYER DISTINCTION (T-357) — why `key` is NOT `mission.schema.json#/$defs/factionKey` ═══

That `$def` is `^[a-z][a-z0-9_]*$`. It is CORRECT, it is already ENFORCED, and it belongs to the OTHER layer. Applying it here would reject a valid mission on every save.

THIS schema describes the AUTHOR'S RAW GRAPH, whose faction-key vocabulary is UPPERCASE by construction: `dto.rs` `FACTION_SIDES = [BLUFOR, OPFOR, INDFOR, CIV]` ("the four canonical faction sides"), and `editor_ops.rs` `ensure_side_faction` sets `key = side` after gating on `matches!(side, "BLUFOR" | "OPFOR" | "INDFOR")`. `faction-library.schema.json` `side` pins the same four as an enum. So the lowercase pattern would reject 100% of what the live Mission Creator can produce, plus `BLUFOR` / `INDFOR` / `USA` across the committed integration payloads, plus 6 of the 39 faction rows in the live database. Nor is the vocabulary closed at this layer — the API accepts `USA`, which is in no enum — so an enum would be wrong here too.

`mission.schema.json` describes the COMPILED game-server document, where that pattern is right and already satisfied by `flatten.rs` `slug_key` — whose own doc says "Lowercase into the schema's `^[a-z][a-z0-9_]*$` pattern". It slugs `factions[].key` into the compiled `orbat` map, `slots[].faction` and `briefings` (all four goldens key those three identically, T-202), and its comment anticipates exactly this layer's input: "Two rows can slug onto one faction (`BLUFOR` and `blufor`), which the editor does not prevent." Nothing is missing on that side.

`orbat_slots.faction` is a THIRD thing, and no schema describes it — correctly, because it is a DB column, not a wire document. `orbat.rs` `derive_orbat_from_editor` copies THIS field verbatim (`faction: f.key.clone()`) into `OrbatSquadTemplate`, which `events.rs` binds straight in. That is why the live table holds uppercase `BLUFOR`/`OPFOR`: not drift, but this layer's vocabulary arriving unchanged, as designed. (16 of those 18 rows are the committed `content_golden.sql` dev seed, inserted directly.)

═══ WHAT IS CONSTRAINED, AND WHY ONLY THIS ═══

`required: [key]` + `minLength: 1`, and nothing else. An ABSENT or EMPTY key is not a vocabulary question — it is broken in one direction only: `OrbatSquadTemplate.faction` is `#[serde(default)]`, so it lands `orbat_slots.faction = ""`, which matches no armory group and renders an Event Hub dossier card with ZERO items. That is the user-visible failure T-346 fixed from the armory side, arriving from the opposite one. Rejecting it here kills it at the write boundary, before it can reach the table.

This REJECTS; it does not transform. Both sides of that join must keep storing bytes verbatim until T-356 lands — T-346 chose require-and-refuse over trimming precisely because this side does not normalise, and a one-sided trim would break the case where ORBAT `"  USA  "` and armory `"  USA  "` agree and render correctly today.

Deliberately NOT constrained: whitespace padding. That is T-356's, in Rust, at ONE site. A regex here too would put one rule in two languages whose definitions differ (ECMA-262 `\s` is a strict superset of Rust's `char::is_whitespace` — U+FEFF), and T-346's lesson is that the bug is DISAGREEMENT between two sites, not the untrimmed value. No schema in this corpus expresses a padding rule; `minLength: 1` is the house form for non-empty.

Also NOT covered: the explicit top-level `orbat[]`, the OTHER input to `parse_orbat_template` (it wins when present). It is absent from all 128 live payloads — Save Version omits it (T-062.1.1) — but it is a second door to the same column, and closing it is T-356's job, not a schema's.

═══ MEASURED BEFORE TIGHTENING ═══

All 39 `editor.factions[]` rows in the live database pass (0 missing, 0 empty, 0 padded, 0 non-string, 0 non-object), as do all 128 live payloads and every committed fixture. The five golden missions are COMPILED documents, validated against `mission.schema.json` and not against this file at all — they fail this schema on `schemaVersion` (string vs integer) and `orbat` (object vs array) both before and after this change, which is itself the clearest evidence that these are two namespaces.

`additionalProperties` is deliberately LEFT OPEN: the row is the wire for authored per-faction briefing prose (`briefing.{situation,mission,execution,markers}`, T-214) and `MissionDocCore::hydrate` `load_row`s every non-`id` field back verbatim, so closing it would make the graph lossy on reload and break the next slice that adds a field before this schema hears about it.*/
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "One authored faction row.\n\n═══ THE LAYER DISTINCTION (T-357) — why `key` is NOT `mission.schema.json#/$defs/factionKey` ═══\n\nThat `$def` is `^[a-z][a-z0-9_]*$`. It is CORRECT, it is already ENFORCED, and it belongs to the OTHER layer. Applying it here would reject a valid mission on every save.\n\nTHIS schema describes the AUTHOR'S RAW GRAPH, whose faction-key vocabulary is UPPERCASE by construction: `dto.rs` `FACTION_SIDES = [BLUFOR, OPFOR, INDFOR, CIV]` (\"the four canonical faction sides\"), and `editor_ops.rs` `ensure_side_faction` sets `key = side` after gating on `matches!(side, \"BLUFOR\" | \"OPFOR\" | \"INDFOR\")`. `faction-library.schema.json` `side` pins the same four as an enum. So the lowercase pattern would reject 100% of what the live Mission Creator can produce, plus `BLUFOR` / `INDFOR` / `USA` across the committed integration payloads, plus 6 of the 39 faction rows in the live database. Nor is the vocabulary closed at this layer — the API accepts `USA`, which is in no enum — so an enum would be wrong here too.\n\n`mission.schema.json` describes the COMPILED game-server document, where that pattern is right and already satisfied by `flatten.rs` `slug_key` — whose own doc says \"Lowercase into the schema's `^[a-z][a-z0-9_]*$` pattern\". It slugs `factions[].key` into the compiled `orbat` map, `slots[].faction` and `briefings` (all four goldens key those three identically, T-202), and its comment anticipates exactly this layer's input: \"Two rows can slug onto one faction (`BLUFOR` and `blufor`), which the editor does not prevent.\" Nothing is missing on that side.\n\n`orbat_slots.faction` is a THIRD thing, and no schema describes it — correctly, because it is a DB column, not a wire document. `orbat.rs` `derive_orbat_from_editor` copies THIS field verbatim (`faction: f.key.clone()`) into `OrbatSquadTemplate`, which `events.rs` binds straight in. That is why the live table holds uppercase `BLUFOR`/`OPFOR`: not drift, but this layer's vocabulary arriving unchanged, as designed. (16 of those 18 rows are the committed `content_golden.sql` dev seed, inserted directly.)\n\n═══ WHAT IS CONSTRAINED, AND WHY ONLY THIS ═══\n\n`required: [key]` + `minLength: 1`, and nothing else. An ABSENT or EMPTY key is not a vocabulary question — it is broken in one direction only: `OrbatSquadTemplate.faction` is `#[serde(default)]`, so it lands `orbat_slots.faction = \"\"`, which matches no armory group and renders an Event Hub dossier card with ZERO items. That is the user-visible failure T-346 fixed from the armory side, arriving from the opposite one. Rejecting it here kills it at the write boundary, before it can reach the table.\n\nThis REJECTS; it does not transform. Both sides of that join must keep storing bytes verbatim until T-356 lands — T-346 chose require-and-refuse over trimming precisely because this side does not normalise, and a one-sided trim would break the case where ORBAT `\"  USA  \"` and armory `\"  USA  \"` agree and render correctly today.\n\nDeliberately NOT constrained: whitespace padding. That is T-356's, in Rust, at ONE site. A regex here too would put one rule in two languages whose definitions differ (ECMA-262 `\\s` is a strict superset of Rust's `char::is_whitespace` — U+FEFF), and T-346's lesson is that the bug is DISAGREEMENT between two sites, not the untrimmed value. No schema in this corpus expresses a padding rule; `minLength: 1` is the house form for non-empty.\n\nAlso NOT covered: the explicit top-level `orbat[]`, the OTHER input to `parse_orbat_template` (it wins when present). It is absent from all 128 live payloads — Save Version omits it (T-062.1.1) — but it is a second door to the same column, and closing it is T-356's job, not a schema's.\n\n═══ MEASURED BEFORE TIGHTENING ═══\n\nAll 39 `editor.factions[]` rows in the live database pass (0 missing, 0 empty, 0 padded, 0 non-string, 0 non-object), as do all 128 live payloads and every committed fixture. The five golden missions are COMPILED documents, validated against `mission.schema.json` and not against this file at all — they fail this schema on `schemaVersion` (string vs integer) and `orbat` (object vs array) both before and after this change, which is itself the clearest evidence that these are two namespaces.\n\n`additionalProperties` is deliberately LEFT OPEN: the row is the wire for authored per-faction briefing prose (`briefing.{situation,mission,execution,markers}`, T-214) and `MissionDocCore::hydrate` `load_row`s every non-`id` field back verbatim, so closing it would make the graph lossy on reload and break the next slice that adds a field before this schema hears about it.",
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
///      "description": "The authored win rule (T-936.1) — the FIRST member of the AUTHORED_BLOCKS passthrough, and the reason that passthrough exists.\n\n═══ THE PASSTHROUGH, AND WHY IT IS A LIST RATHER THAN A FUNCTION PER BLOCK ═══\n\n`crates/map-engine-core/src/mission/extensions.rs` holds `AUTHORED_BLOCKS`: one row per top-level block the editor authors and the compile carries VERBATIM, each with a typed validator. `compile_payload` copies every listed key from the document's `meta` bag onto this payload root in one loop, and `flatten_to_mod_document` reads them back through the same list — so the six sibling slices that follow T-936.1 (`tasks`, `radioPlan`, `weatherTimeline`, `audio`, `spawnModules`, `tacticalGraphics`) each add ONE row plus ONE validator and touch `compile.rs` not at all — that file's copy loop is generic over the list, which is the whole point: it is contested by five other tickets, and seven slices each adding a hand-written copy line is seven merge conflicts and seven chances to forget one.\n\n═══ CORRECTION (T-936.7 / T-946.53) — THE ROW IS NOT A WIRE ═══\n\nThis paragraph used to read \"touch neither `compile.rs` nor `flatten.rs`\", and the `flatten.rs` half was FALSE. `extensions.rs`'s own module header said the same thing in the same words; both are corrected in the commit that falsifies them. `flatten_to_mod_document` reads the blocks back through `EditorPayload`, a NAMED-FIELD deserialise struct with NO `#[serde(flatten)]` catch-all (deliberately — flatten makes serde buffer every unmatched top-level key of an 8 MB payload into `Content` rather than skipping it with `IgnoredAny`), and through `EditorPayload::authored_block_value`, a hand `match` on the key ending `_ => None`. A block with a row in `AUTHORED_BLOCKS` but no named field and no match arm is therefore dropped in complete SILENCE: the compile succeeds, no diagnostic is raised, and `/compiled` simply does not contain what the author wrote. That went live for `weatherTimeline` and `audio` (T-946.44) before anyone noticed. So a new block moves in TWO files: a row in `extensions.rs`, AND a field plus an arm in `flatten.rs`. `flatten.rs`'s `every_authored_block_key_reaches_the_wire` walks `AUTHORED_BLOCKS` itself and now fails by name on a row missing either half.\n\nA block only reaches this root if the ROW EXISTS. An unlisted key stays parked in `payloadExtras` (T-219) exactly as it does today, so this is not an open passthrough wearing a list.\n\n═══ WHY THE ROOT IS STILL OPEN ═══\n\nUnchanged, and deliberately so — see `$defs/editorFaction`'s note. Declaring `winConditions` here makes it a KNOWN part of this contract instead of something the open root happens to tolerate (the argument `zones` records above); it does not close the root, and closing it would make the graph lossy on reload and break the next slice that adds a key before this schema hears about it.\n\n═══ WHY `mode` IS FIVE VALUES HERE AND SEVEN IN mission.schema.json ═══\n\nThese are the five the editor can author. The compiled contract's enum also carries `points_then_attrition` and `defender_holds_or_attacker_destroys`, which are grandfathered hand-authored golden values that no editor payload can produce — the layer distinction `$defs/editorFaction` makes for `key`, applied to `mode`. Narrowing here is therefore free: it rejects nothing the editor can emit, and it catches a hand-staged payload that invents a sixth rule at the write boundary rather than at `/compiled`.",
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
///      "description": "Authored play-area / objective zones, carried into the compiled document by `flatten.rs` `derive_zones`. Declared here so the key is a KNOWN part of this contract rather than something the open root happens to tolerate; the root's openness is what let it go unmentioned for as long as there was no draw tool.\n\n═══ WHY THERE IS NO PER-ITEM SUBSCHEMA HERE (T-581) ═══\n\nNot the O(n) argument that keeps `editor.slots` bare — `zones` is BOUNDED (a play area plus a handful of objectives), so cost would not decide it. The reason is the one `mission.schema.json#/$defs/zoneRules` states about itself: the rule vocabulary is closed BECAUSE both mod readers are typed and cannot tell `graceSecconds` from \"authored nothing\", which makes that `$def` the ONLY place a misspelled rule key can be caught. Restating `type`'s six-value enum or the rule names here would create the second place — and then the two would drift, which is the T-346 lesson (the bug was DISAGREEMENT between two sites, not either site's rule) and the drift T-367 refused to reopen for the editor graph.\n\n═══ WHERE IT IS ENFORCED INSTEAD ═══\n\n`apps/website/api_v2/src/missions/contract/zone_quantisation.rs` `scan_authored_zones`, on the same `details` array as the T-181.44 wire-safety and T-416 cargo walks, so a bad zone is a 400 the author sees while the editor is still open. Its verdict is `mission.schema.json#/$defs/zone` itself — lifted out of the bytes already embedded for `validate_mission_document` — so \"save accepted it\" and \"`/compiled` will validate\" are one sentence rather than two kept in step.\n\nIt runs the POST-QUANTISATION row, which a schema on THIS document provably cannot. `flatten.rs` rounds every zone coordinate to 0.1 m and tests `r > 0.0` BEFORE rounding, so `round_coord(0.04) = 0.0` and a radius that satisfies `$defs/circle.r` `exclusiveMinimum: 0` on the way in violates it on the way out. MEASURED over the real HTTP path: `circle r: 0.04` saved 201 and 500'd `/compiled` permanently, as did `rules: {\"notInT241Vocabulary\": 1}` and `type: \"capture\"`. A click without a drag in the draw tool produces exactly that radius.\n\nTwo keys are deliberately NOT checked at save even so. `faction` is uppercase at THIS layer and slugged by `flatten.rs` `slug_key` into the compiled `$defs/factionKey` — the T-357 distinction, and validating the authored form would reject missions that compile perfectly. And a zone the compile DROPS (no usable shape, empty `id`, empty `type`) is left alone: it never reaches a game server, so refusing it would narrow the accept set below the compile set, which is the invariant T-367 pinned.",
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
    /**Authored play-area / objective zones, carried into the compiled document by `flatten.rs` `derive_zones`. Declared here so the key is a KNOWN part of this contract rather than something the open root happens to tolerate; the root's openness is what let it go unmentioned for as long as there was no draw tool.

    ═══ WHY THERE IS NO PER-ITEM SUBSCHEMA HERE (T-581) ═══

    Not the O(n) argument that keeps `editor.slots` bare — `zones` is BOUNDED (a play area plus a handful of objectives), so cost would not decide it. The reason is the one `mission.schema.json#/$defs/zoneRules` states about itself: the rule vocabulary is closed BECAUSE both mod readers are typed and cannot tell `graceSecconds` from "authored nothing", which makes that `$def` the ONLY place a misspelled rule key can be caught. Restating `type`'s six-value enum or the rule names here would create the second place — and then the two would drift, which is the T-346 lesson (the bug was DISAGREEMENT between two sites, not either site's rule) and the drift T-367 refused to reopen for the editor graph.

    ═══ WHERE IT IS ENFORCED INSTEAD ═══

    `apps/website/api_v2/src/missions/contract/zone_quantisation.rs` `scan_authored_zones`, on the same `details` array as the T-181.44 wire-safety and T-416 cargo walks, so a bad zone is a 400 the author sees while the editor is still open. Its verdict is `mission.schema.json#/$defs/zone` itself — lifted out of the bytes already embedded for `validate_mission_document` — so "save accepted it" and "`/compiled` will validate" are one sentence rather than two kept in step.

    It runs the POST-QUANTISATION row, which a schema on THIS document provably cannot. `flatten.rs` rounds every zone coordinate to 0.1 m and tests `r > 0.0` BEFORE rounding, so `round_coord(0.04) = 0.0` and a radius that satisfies `$defs/circle.r` `exclusiveMinimum: 0` on the way in violates it on the way out. MEASURED over the real HTTP path: `circle r: 0.04` saved 201 and 500'd `/compiled` permanently, as did `rules: {"notInT241Vocabulary": 1}` and `type: "capture"`. A click without a drag in the draw tool produces exactly that radius.

    Two keys are deliberately NOT checked at save even so. `faction` is uppercase at THIS layer and slugged by `flatten.rs` `slug_key` into the compiled `$defs/factionKey` — the T-357 distinction, and validating the authored form would reject missions that compile perfectly. And a zone the compile DROPS (no usable shape, empty `id`, empty `type`) is left alone: it never reaches a game server, so refusing it would narrow the accept set below the compile set, which is the invariant T-367 pinned.*/
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
/**The authored win rule (T-936.1) — the FIRST member of the AUTHORED_BLOCKS passthrough, and the reason that passthrough exists.

═══ THE PASSTHROUGH, AND WHY IT IS A LIST RATHER THAN A FUNCTION PER BLOCK ═══

`crates/map-engine-core/src/mission/extensions.rs` holds `AUTHORED_BLOCKS`: one row per top-level block the editor authors and the compile carries VERBATIM, each with a typed validator. `compile_payload` copies every listed key from the document's `meta` bag onto this payload root in one loop, and `flatten_to_mod_document` reads them back through the same list — so the six sibling slices that follow T-936.1 (`tasks`, `radioPlan`, `weatherTimeline`, `audio`, `spawnModules`, `tacticalGraphics`) each add ONE row plus ONE validator and touch `compile.rs` not at all — that file's copy loop is generic over the list, which is the whole point: it is contested by five other tickets, and seven slices each adding a hand-written copy line is seven merge conflicts and seven chances to forget one.

═══ CORRECTION (T-936.7 / T-946.53) — THE ROW IS NOT A WIRE ═══

This paragraph used to read "touch neither `compile.rs` nor `flatten.rs`", and the `flatten.rs` half was FALSE. `extensions.rs`'s own module header said the same thing in the same words; both are corrected in the commit that falsifies them. `flatten_to_mod_document` reads the blocks back through `EditorPayload`, a NAMED-FIELD deserialise struct with NO `#[serde(flatten)]` catch-all (deliberately — flatten makes serde buffer every unmatched top-level key of an 8 MB payload into `Content` rather than skipping it with `IgnoredAny`), and through `EditorPayload::authored_block_value`, a hand `match` on the key ending `_ => None`. A block with a row in `AUTHORED_BLOCKS` but no named field and no match arm is therefore dropped in complete SILENCE: the compile succeeds, no diagnostic is raised, and `/compiled` simply does not contain what the author wrote. That went live for `weatherTimeline` and `audio` (T-946.44) before anyone noticed. So a new block moves in TWO files: a row in `extensions.rs`, AND a field plus an arm in `flatten.rs`. `flatten.rs`'s `every_authored_block_key_reaches_the_wire` walks `AUTHORED_BLOCKS` itself and now fails by name on a row missing either half.

A block only reaches this root if the ROW EXISTS. An unlisted key stays parked in `payloadExtras` (T-219) exactly as it does today, so this is not an open passthrough wearing a list.

═══ WHY THE ROOT IS STILL OPEN ═══

Unchanged, and deliberately so — see `$defs/editorFaction`'s note. Declaring `winConditions` here makes it a KNOWN part of this contract instead of something the open root happens to tolerate (the argument `zones` records above); it does not close the root, and closing it would make the graph lossy on reload and break the next slice that adds a key before this schema hears about it.

═══ WHY `mode` IS FIVE VALUES HERE AND SEVEN IN mission.schema.json ═══

These are the five the editor can author. The compiled contract's enum also carries `points_then_attrition` and `defender_holds_or_attacker_destroys`, which are grandfathered hand-authored golden values that no editor payload can produce — the layer distinction `$defs/editorFaction` makes for `key`, applied to `mode`. Narrowing here is therefore free: it rejects nothing the editor can emit, and it catches a hand-staged payload that invents a sixth rule at the write boundary rather than at `/compiled`.*/
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "The authored win rule (T-936.1) — the FIRST member of the AUTHORED_BLOCKS passthrough, and the reason that passthrough exists.\n\n═══ THE PASSTHROUGH, AND WHY IT IS A LIST RATHER THAN A FUNCTION PER BLOCK ═══\n\n`crates/map-engine-core/src/mission/extensions.rs` holds `AUTHORED_BLOCKS`: one row per top-level block the editor authors and the compile carries VERBATIM, each with a typed validator. `compile_payload` copies every listed key from the document's `meta` bag onto this payload root in one loop, and `flatten_to_mod_document` reads them back through the same list — so the six sibling slices that follow T-936.1 (`tasks`, `radioPlan`, `weatherTimeline`, `audio`, `spawnModules`, `tacticalGraphics`) each add ONE row plus ONE validator and touch `compile.rs` not at all — that file's copy loop is generic over the list, which is the whole point: it is contested by five other tickets, and seven slices each adding a hand-written copy line is seven merge conflicts and seven chances to forget one.\n\n═══ CORRECTION (T-936.7 / T-946.53) — THE ROW IS NOT A WIRE ═══\n\nThis paragraph used to read \"touch neither `compile.rs` nor `flatten.rs`\", and the `flatten.rs` half was FALSE. `extensions.rs`'s own module header said the same thing in the same words; both are corrected in the commit that falsifies them. `flatten_to_mod_document` reads the blocks back through `EditorPayload`, a NAMED-FIELD deserialise struct with NO `#[serde(flatten)]` catch-all (deliberately — flatten makes serde buffer every unmatched top-level key of an 8 MB payload into `Content` rather than skipping it with `IgnoredAny`), and through `EditorPayload::authored_block_value`, a hand `match` on the key ending `_ => None`. A block with a row in `AUTHORED_BLOCKS` but no named field and no match arm is therefore dropped in complete SILENCE: the compile succeeds, no diagnostic is raised, and `/compiled` simply does not contain what the author wrote. That went live for `weatherTimeline` and `audio` (T-946.44) before anyone noticed. So a new block moves in TWO files: a row in `extensions.rs`, AND a field plus an arm in `flatten.rs`. `flatten.rs`'s `every_authored_block_key_reaches_the_wire` walks `AUTHORED_BLOCKS` itself and now fails by name on a row missing either half.\n\nA block only reaches this root if the ROW EXISTS. An unlisted key stays parked in `payloadExtras` (T-219) exactly as it does today, so this is not an open passthrough wearing a list.\n\n═══ WHY THE ROOT IS STILL OPEN ═══\n\nUnchanged, and deliberately so — see `$defs/editorFaction`'s note. Declaring `winConditions` here makes it a KNOWN part of this contract instead of something the open root happens to tolerate (the argument `zones` records above); it does not close the root, and closing it would make the graph lossy on reload and break the next slice that adds a key before this schema hears about it.\n\n═══ WHY `mode` IS FIVE VALUES HERE AND SEVEN IN mission.schema.json ═══\n\nThese are the five the editor can author. The compiled contract's enum also carries `points_then_attrition` and `defender_holds_or_attacker_destroys`, which are grandfathered hand-authored golden values that no editor payload can produce — the layer distinction `$defs/editorFaction` makes for `key`, applied to `mode`. Narrowing here is therefore free: it rejects nothing the editor can emit, and it catches a hand-staged payload that invents a sixth rule at the write boundary rather than at `/compiled`.",
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
