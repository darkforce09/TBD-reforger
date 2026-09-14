//! Missions: the library card, the full dossier, and the approval trail.
//!
//! **Role:** the rows the mission library lists, the detail a dossier renders, the armoury a
//! mission exposes, and the environment block the compiler reads back out of the row.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** the review stamp fields are absent from the wire on a mission that was never reviewed,
//! so a fixture captured from a reviewed mission is the only thing that proves they round-trip.
//! `MissionDetail` deliberately has no catch-all for unknown keys, so a field the backend stops
//! sending fails the round-trip test instead of quietly rendering as a placeholder.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One mission as the library lists it, including the review stamp an author needs to
/// see why their submission came back.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionCard {
    pub id: String,
    pub title: String,
    pub author_id: String,
    pub terrain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_terrain_name: Option<String>,
    pub game_mode: String,
    pub weather: String,
    pub time_of_day: String,
    pub max_players: i64,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    pub author_name: String,
    pub author_avatar: String,
    /// Why the mission was sent back. The only thing an author is ever told about a rejection,
    /// and absent entirely on a mission that was never reviewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One row of the approvals queue.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRow {
    pub mission_id: String,
    pub title: String,
    pub terrain: String,
    pub author_id: String,
    pub author_name: String,
    pub submitted_at: String,
}

/// A pointer to one saved version of a mission.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionVersionRef {
    pub created_at: String,
    pub created_by: String,
    pub id: String,
    pub json_payload: Value,
    pub mission_id: String,
    pub semver: String,
}

/// A mission in full: its metadata, its review stamp, and the version pointers a
/// dossier lists.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionDetail {
    pub armory: Vec<Value>,
    pub author_avatar: String,
    pub author_id: String,
    pub author_name: String,
    pub bookmarked: bool,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<MissionVersionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version_id: Option<String>,
    /// The mission's shape. Editable after creation — the settings dialog and the create dialog
    /// both write it.
    pub game_mode: String,
    pub id: String,
    /// The player count the author typed when creating the mission.
    ///
    /// This is not the number of seats the document actually contains, and nothing reconciles the
    /// two; the derived placed-slot count is what the editor displays. This value is what the
    /// compiler copies into the compiled mission, which is why it stays. There is deliberately no
    /// minimum counterpart — no column, model field or handler has ever had one.
    pub max_players: i64,
    pub status: String,
    pub terrain: String,
    pub time_of_day: String,
    pub title: String,
    pub updated_at: String,
    pub weather: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_terrain_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// Why the mission was sent back, and absent entirely when it was never reviewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
}

impl MissionDetail {
    /// Project this row onto the metadata the shared mission compiler reads.
    ///
    /// The one input the editor's export cannot derive from the document itself. Returning the
    /// struct rather than a hand-built JSON object means there is no key to mistype. Note that the
    /// `author` field carries the account id, not the display name — the display name travels
    /// beside it — which is the one field a reasonable reading gets wrong, and getting it wrong
    /// would produce a document that looks right and is not the one the server builds.
    pub fn compiled_meta(&self) -> map_engine_core::mission::flatten::MissionMeta {
        map_engine_core::mission::flatten::MissionMeta {
            id: self.id.clone(),
            title: self.title.clone(),
            author: self.author_id.clone(),
            terrain: self.terrain.clone(),
            custom_terrain_name: self.custom_terrain_name.clone().unwrap_or_default(),
            max_players: self.max_players,
            time_of_day: self.time_of_day.clone(),
            weather_preset: self.weather.clone(),
        }
    }
}

/// One item a mission's armoury offers.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmoryItem {
    pub id: String,
    pub item_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One faction's slice of a mission's armoury.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmoryFaction {
    pub faction: String,
    pub items: Vec<ArmoryItem>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// The environment block of a mission row: time of day and weather as authored.
///
/// The document's own environment takes precedence downstream, so these are copied across
/// rather than reconciled here.
#[derive(Clone, Debug, PartialEq)]
pub struct MissionEnv {
    pub terrain: String,
    pub time: String,
    pub weather: String,
    // Render preferences are per-mission and live in the environment block. The per-viewer
    // basemap and world-layer toggles are separate and live in browser storage.
    pub show_hillshade: bool,
    pub hillshade_opacity: f64,
    pub show_grid: bool,
}

/// The neutral environment a mission starts from when the row carries none.
impl Default for MissionEnv {
    fn default() -> Self {
        Self {
            terrain: String::new(),
            time: String::new(),
            weather: String::new(),
            show_hillshade: true,
            hillshade_opacity: 0.4,
            show_grid: true,
        }
    }
}
