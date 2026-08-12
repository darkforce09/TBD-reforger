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
///      "description": "Lossless editor graph. `squads`, `slots` and `editorLayers` are intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots; the rules those arrays do need are expressed in CODE instead — `crates/map-engine-core/src/mission/wire_safety.rs`, whose header records the measurement (615.6 ms parse vs 34.5 ms scan at 367k slots). `factions` is the one array that is BOUNDED — see its own note — so a per-item subschema there is a different cost class from the thing that rule refuses.",
///      "type": "object",
///      "properties": {
///        "editorLayers": {
///          "type": "array"
///        },
///        "factions": {
///          "description": "Faction rows, carried VERBATIM from the document core (`compile_payload` clones `factionsById` whole). Bounded in practice: the editor mints at most one row per side (`ensure_side_faction` → `add_faction(faction-{SIDE}, key=SIDE, name=SIDE)`), gated on three literals, and the largest live payload holds ONE. So `items` here costs O(factions) ≤ 4, not O(slots) — which is why this array carries a subschema and its three siblings deliberately do not.",
///          "type": "array",
///          "items": {
///            "$ref": "#/$defs/editorFaction"
///          }
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
///Lossless editor graph. `squads`, `slots` and `editorLayers` are intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots; the rules those arrays do need are expressed in CODE instead — `crates/map-engine-core/src/mission/wire_safety.rs`, whose header records the measurement (615.6 ms parse vs 34.5 ms scan at 367k slots). `factions` is the one array that is BOUNDED — see its own note — so a per-item subschema there is a different cost class from the thing that rule refuses.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Lossless editor graph. `squads`, `slots` and `editorLayers` are intentionally unconstrained (no per-item schema) so validation stays O(1) on missions with hundreds of thousands of slots; the rules those arrays do need are expressed in CODE instead — `crates/map-engine-core/src/mission/wire_safety.rs`, whose header records the measurement (615.6 ms parse vs 34.5 ms scan at 367k slots). `factions` is the one array that is BOUNDED — see its own note — so a per-item subschema there is a different cost class from the thing that rule refuses.",
///  "type": "object",
///  "properties": {
///    "editorLayers": {
///      "type": "array"
///    },
///    "factions": {
///      "description": "Faction rows, carried VERBATIM from the document core (`compile_payload` clones `factionsById` whole). Bounded in practice: the editor mints at most one row per side (`ensure_side_faction` → `add_faction(faction-{SIDE}, key=SIDE, name=SIDE)`), gated on three literals, and the largest live payload holds ONE. So `items` here costs O(factions) ≤ 4, not O(slots) — which is why this array carries a subschema and its three siblings deliberately do not.",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/editorFaction"
///      }
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
    ///Faction rows, carried VERBATIM from the document core (`compile_payload` clones `factionsById` whole). Bounded in practice: the editor mints at most one row per side (`ensure_side_faction` → `add_faction(faction-{SIDE}, key=SIDE, name=SIDE)`), gated on three literals, and the largest live payload holds ONE. So `items` here costs O(factions) ≤ 4, not O(slots) — which is why this array carries a subschema and its three siblings deliberately do not.
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub factions: ::std::vec::Vec<EditorFaction>,
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
