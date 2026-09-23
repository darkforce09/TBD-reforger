// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-editor-payload.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{EditorFaction, EditorLayer, EditorSlot};

///The 2D-editor 'superset' stored verbatim as a MissionVersion.json_payload (the write side of POST /api/v1/missions/:id/versions; mirrors the frontend compile.ts MissionPayload). This is NOT the canonical mission.schema.json document — that is the game-server contract derived/exported separately. Its integer schemaVersion is the editor-payload format version, a DISTINCT namespace from the canonical mission contract's string schemaVersion. Validation is intentionally lenient on presence (minimal and partial saves are valid, including the empty {} a freshly created mission stores) but strict on type, to reject malformed payloads and the schemaVersion namespace confusion (a string here) before persist.
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
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
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for TbdMissionEditorPayloadWinConditionsEndOnItem
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for TbdMissionEditorPayloadWinConditionsEndOnItem
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`TbdMissionEditorPayloadWinConditionsExtractionZoneId`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for TbdMissionEditorPayloadWinConditionsExtractionZoneId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for TbdMissionEditorPayloadWinConditionsExtractionZoneId
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for TbdMissionEditorPayloadWinConditionsExtractionZoneId
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`TbdMissionEditorPayloadWinConditionsMode`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
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
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TbdMissionEditorPayloadWinConditionsMode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TbdMissionEditorPayloadWinConditionsMode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`TbdMissionEditorPayloadWinConditionsVipSlotId`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for TbdMissionEditorPayloadWinConditionsVipSlotId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for TbdMissionEditorPayloadWinConditionsVipSlotId
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for TbdMissionEditorPayloadWinConditionsVipSlotId
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
