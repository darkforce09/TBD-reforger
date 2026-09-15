//! Role: scenario.
//! Position: `mission/ast` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::entities::ModNet;
use serde::Serialize;

/// `mission.schema.json#/$defs/radioPlan`. `nets` carries `minItems: 1`, so an empty plan is a schema violation rather than an empty block — the whole key is omitted instead (`radioPlan` is not in the schema's top-level `required`, and `TBD_RadioPlan.Parse` treats an absent plan as legal and logs `nets=0`).
#[derive(Debug, Serialize)]
pub struct ModRadioPlan {
    /// Nets.
    pub nets: Vec<ModNet>,
}

/// Domain representation of mod circle.
#[derive(Debug, Serialize)]
pub struct ModCircle {
    /// X.
    pub x: f64,
    /// Z.
    pub z: f64,
    /// R.
    pub r: f64,
}

/// Compiled zone geometry — `mission.schema.json#/$defs/shape` (`oneOf` circle | polygon).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ModZoneShape {
    /// Domain representation of circle.
    Circle { circle: ModCircle },
    /// Domain representation of polygon.
    Polygon { polygon: Vec<[f64; 2]> },
}

/// Domain representation of mod zone.
#[derive(Debug, Serialize)]
pub struct ModZone {
    /// Id.
    pub id: String,
    /// Kind.
    #[serde(rename = "type")]
    pub kind: String,
    /// Label.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// Faction.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub faction: String,
    /// Shape.
    pub shape: ModZoneShape,

    /// Zone-type rules (`mission.schema.json#/$defs/zoneRules`) — passed through verbatim when authored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<serde_json::Value>,
}

/// Domain representation of mod faction.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFaction {
    /// Key.
    pub key: String,
    /// Display name.
    pub display_name: String,
    /// Preset id.
    pub preset_id: String,
    /// Tickets.
    pub tickets: i64,
}

/// Domain representation of mod meta.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMeta {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Author.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// Terrain.
    pub terrain: String,
    /// Template id.
    pub template_id: String,
    /// Player range.
    pub player_range: [i64; 2],
}

/// Domain representation of mod environment.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEnvironment {
    /// Date time.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date_time: String,
    /// Weather preset.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub weather_preset: String,

    /// Wind dir deg.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_dir_deg: Option<f64>,

    /// Fog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fog: Option<f64>,

    /// Wind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind: Option<f64>,

    /// View distance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_distance: Option<f64>,
}

/// Domain representation of mod flow.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFlow {
    /// Briefing seconds.
    pub briefing_seconds: i64,
    /// Safe start seconds.
    pub safe_start_seconds: i64,
    /// Time limit seconds.
    pub time_limit_seconds: i64,
    /// Jip.
    pub jip: String,
}

/// `mission.schema.json#/$defs/winConditions` — the mission's win rule.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModWinConditions {
    /// Mode.
    pub mode: String,
    /// End on.
    pub end_on: Vec<String>,
    /// Params.
    #[serde(flatten)]
    pub params: crate::mission::win_conditions::WinConditionParams,
}

/// **PASSED THROUGH, never derived.** Respawn / spectator / NVG are authored policy, not something the ORBAT can invent. Every field is optional (`$defs/settings` declares no `required`); an authored `"settings": {}` still reaches the wire as `{}` so "present but empty" stays distinct from "key absent" for the mod reader (`TBD_MissionSettingsStruct`).
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModSettings {
    /// Respawn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respawn: Option<String>,
    /// Spectator policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectator_policy: Option<String>,
    /// Night vision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub night_vision: Option<bool>,
}

/// One `briefing.markers[]` entry (`mission.schema.json#/$defs/marker`).
#[derive(Debug, Serialize)]
pub struct ModMarker {
    /// X.
    pub x: f64,
    /// Z.
    pub z: f64,
    /// Icon.
    pub icon: String,

    /// Capped at [`MOD_MAX_MARKER_LABEL_CHARS`] here so the mod never has to — the same reason [`ModNet::label`] is capped: `TBD_MarkerService.CapLabel` truncates without telling anyone, and the compiled document a human can read should already show the string the player sees.
    pub label: String,
}

/// One `briefings` entry (`mission.schema.json#/$defs/briefing`), keyed by faction.
#[derive(Debug, Default, Serialize)]
pub struct ModBriefing {
    /// Situation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub situation: Option<String>,
    /// Mission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission: Option<String>,
    /// Execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<String>,
    /// Markers.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<ModMarker>,
}
