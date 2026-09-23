//! Missions: the library card, the full dossier, the saved version, and the approval queue row.
//!
//! **Role:** the rows the mission library lists, the bare row a submission or a review decision
//! answers with, the detail a dossier renders, one immutable saved version, the armoury a mission
//! exposes, the environment block the compiler reads back out of the row, and the row the approvals
//! queue lists for each mission awaiting review.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** the review stamp fields are absent from the wire on a mission that was never reviewed,
//! so a fixture captured from a reviewed mission is the only thing that proves they round-trip.
//! `approved_artifact_id` names the artifact the latest approval decided and is absent on a mission
//! never approved from an artifact. `MissionDetail` deliberately has no catch-all for unknown keys,
//! so a field the backend stops sending fails the round-trip test instead of quietly rendering as a
//! placeholder.

use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use website_map_engine::data::store::operations::environment::MissionEnv;

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
    /// Why the mission was sent back. Absent entirely on a mission that was never rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    /// The artifact the latest approval decided: exactly the bytes a deployment of this mission
    /// loads. Absent on a mission never approved from an artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_artifact_id: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// A mission row as submission and both review decisions answer it: the library card without its
/// author decoration and bookmark.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionRow {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// One row of the approvals queue: a mission awaiting review, and the review under way.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRow {
    pub mission_id: String,
    pub title: String,
    pub terrain: String,
    pub author_id: String,
    pub author_name: String,
    /// When the pending review opened; for a mission that predates reviews, when the mission was
    /// last updated.
    pub submitted_at: String,
    /// The pending review. This and the three fields below are absent together for a mission
    /// submitted before artifacts existed, which is decided only after its author resubmits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_id: Option<String>,
    /// The artifact under review — the one a decision must name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    /// That artifact's SHA-256 digest, lowercase hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_digest: Option<String>,
    /// The version the artifact compiled from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_semver: Option<String>,
}

/// One immutable saved version of a mission: its number, the authored editor payload, and who
/// saved it when.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionVersion {
    pub created_at: String,
    pub created_by: String,
    pub id: String,
    pub json_payload: Value,
    pub mission_id: String,
    pub semver: String,
    /// The note the author saved the version with; absent when they left none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_notes: Option<String>,
}

/// A mission in full: its metadata, its review stamp, and the version a dossier lists.
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
    pub current_version: Option<MissionVersion>,
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
    /// Why the mission was sent back, and absent entirely when it was never rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    /// The artifact the latest approval decided; absent on a mission never approved from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_artifact_id: Option<String>,
}

impl MissionDetail {
    /// Project this row onto the metadata the shared mission compiler reads.
    ///
    /// The one input the editor's export cannot derive from the document itself. Returning the
    /// struct rather than a hand-built JSON object means there is no key to mistype. Note that the
    /// `author` field carries the account id, not the display name — the display name travels
    /// beside it — which is the one field a reasonable reading gets wrong, and getting it wrong
    /// would produce a document that looks right and is not the one the server builds.
    pub fn compiled_meta(&self) -> website_map_engine::data::scenario::flatten::MissionMeta {
        website_map_engine::data::scenario::flatten::MissionMeta {
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
