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
//! @contract mission-library.schema.json#/definitions/MissionLibraryPage
//! @contract mission-library.schema.json#/definitions/MissionDetail

use super::identifiers::{
    DiscordUserId, MissionArmoryId, MissionArtifactId, MissionId, MissionReviewId, MissionVersionId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One mission as the library lists it, including the review stamp an author needs to
/// see why their submission came back.
/// @contract mission-library.schema.json#/definitions/MissionCard
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionCard {
    /// The record's id.
    pub id: MissionId,
    /// Display title shown in the library.
    pub title: String,
    /// Discord id of the mission author.
    pub author_id: DiscordUserId,
    /// Terrain the mission plays on.
    pub terrain: String,
    /// Terrain name when `terrain` is a custom terrain; empty (omitted on the wire) otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_terrain_name: Option<String>,
    /// Game mode the mission is played as.
    pub game_mode: String,
    /// Weather preset.
    pub weather: String,
    /// The in-game time of day the mission starts at, as `HH:MM:SS`.
    pub time_of_day: String,
    /// Player capacity of the mission.
    pub max_players: i64,
    /// Lifecycle state of the mission.
    pub status: String,
    /// Library thumbnail URL; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// Briefing text; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    /// The author's display name.
    pub author_name: String,
    /// The author's avatar URL; empty when the author has none.
    pub author_avatar: String,
    /// Why the mission was sent back. Absent entirely on a mission that was never rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    /// Discord id of the reviewer who made the latest review decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    /// When the mission's latest submission was reviewed; absent before any review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    /// The artifact the latest approval decided: exactly the bytes a deployment of this mission
    /// loads. Absent on a mission never approved from an artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_artifact_id: Option<MissionArtifactId>,
    /// Every field the API sends beyond the ones named here, kept so a round trip loses nothing.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// A mission row as submission and both review decisions answer it: the library card without its
/// author decoration and bookmark.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionRow {
    /// The record's id.
    pub id: MissionId,
    /// Display title shown in the library.
    pub title: String,
    /// Discord id of the mission author.
    pub author_id: DiscordUserId,
    /// Terrain the mission plays on.
    pub terrain: String,
    /// Terrain name when `terrain` is a custom terrain; empty (omitted on the wire) otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_terrain_name: Option<String>,
    /// Game mode the mission is played as.
    pub game_mode: String,
    /// Weather preset.
    pub weather: String,
    /// The in-game time of day the mission starts at, as `HH:MM:SS`.
    pub time_of_day: String,
    /// Player capacity of the mission.
    pub max_players: i64,
    /// Lifecycle state of the mission.
    pub status: String,
    /// Library thumbnail URL; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// Briefing text; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    /// Latest saved [`MissionVersion`]; absent before the first save.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version_id: Option<MissionVersionId>,
    /// The artifact the latest approval decided; deployments load exactly these bytes. Absent on a
    /// mission never approved from an artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_artifact_id: Option<MissionArtifactId>,
    /// Reviewer's reason for the latest rejection; empty (omitted on the wire) otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    /// Discord id of the reviewer who made the latest review decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    /// When the mission's latest submission was reviewed; absent before any review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    /// When the mission row was created (RFC 3339 UTC).
    pub created_at: String,
    /// When the mission row last changed (RFC 3339 UTC).
    pub updated_at: String,
}

/// One row of the approvals queue: a mission awaiting review, and the review under way.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRow {
    /// The mission whose thread the comment belongs to.
    pub mission_id: MissionId,
    /// The mission's title.
    pub title: String,
    /// Terrain the mission plays on.
    pub terrain: String,
    /// Discord id of the comment's author.
    pub author_id: DiscordUserId,
    /// The author's display name.
    pub author_name: String,
    /// When the pending review opened; for a mission that predates reviews, when the mission was
    /// last updated.
    pub submitted_at: String,
    /// The pending review. This and the three fields below are absent together for a mission
    /// submitted before artifacts existed, which is decided only after its author resubmits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_id: Option<MissionReviewId>,
    /// The artifact under review — the one a decision must name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<MissionArtifactId>,
    /// That artifact's SHA-256 digest, lowercase hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_digest: Option<String>,
    /// The version the artifact compiled from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_semver: Option<String>,
}

/// One immutable saved version of a mission: its number, the authored editor payload, and who
/// saved it when.
/// @contract mission-library.schema.json#/definitions/MissionVersion
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionVersion {
    /// When this version was saved (RFC 3339 UTC).
    pub created_at: String,
    /// Discord id of the user who saved this version.
    pub created_by: String,
    /// The record's id.
    pub id: MissionVersionId,
    /// The version's mission document, passed through as the API stored it.
    pub json_payload: Value,
    /// The mission this version belongs to.
    pub mission_id: MissionId,
    /// Semantic version string of this snapshot, e.g. `0.1.0`.
    pub semver: String,
    /// The note the author saved the version with; absent when they left none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_notes: Option<String>,
}

/// A mission in full: its metadata, its review stamp, and the version a dossier lists.
/// @contract mission-library.schema.json#/definitions/MissionDetail
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionDetail {
    /// The mission's armory entries, as the API sends them.
    pub armory: Vec<Value>,
    /// The author's avatar URL; empty when the author has none.
    pub author_avatar: String,
    /// Discord id of the mission author.
    pub author_id: DiscordUserId,
    /// The author's display name.
    pub author_name: String,
    /// Whether the caller has bookmarked the mission.
    pub bookmarked: bool,
    /// When the mission row was created (RFC 3339 UTC).
    pub created_at: String,
    /// The mission's current saved version; absent before the first save.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<MissionVersion>,
    /// Latest saved [`MissionVersion`]; absent before the first save.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version_id: Option<MissionVersionId>,
    /// The mission's shape. Editable after creation — the settings dialog and the create dialog
    /// both write it.
    pub game_mode: String,
    /// The record's id.
    pub id: MissionId,
    /// The player count the author typed when creating the mission.
    ///
    /// This is not the number of seats the document actually contains, and nothing reconciles the
    /// two; the derived placed-slot count is what the editor displays. This value is what the
    /// compiler copies into the compiled mission, which is why it stays. There is deliberately no
    /// minimum counterpart — no column, model field or handler has ever had one.
    pub max_players: i64,
    /// Lifecycle state of the mission.
    pub status: String,
    /// Terrain the mission plays on.
    pub terrain: String,
    /// The in-game time of day the mission starts at, as `HH:MM:SS`.
    pub time_of_day: String,
    /// Display title shown in the library.
    pub title: String,
    /// When the mission row last changed (RFC 3339 UTC).
    pub updated_at: String,
    /// Weather preset.
    pub weather: String,
    /// Briefing text; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    /// Terrain name when `terrain` is a custom terrain; empty (omitted on the wire) otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_terrain_name: Option<String>,
    /// Library thumbnail URL; empty (omitted on the wire) when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// Why the mission was sent back, and absent entirely when it was never rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    /// Discord id of the reviewer who made the latest review decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    /// When the mission's latest submission was reviewed; absent before any review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    /// The artifact the latest approval decided; absent on a mission never approved from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_artifact_id: Option<MissionArtifactId>,
}

impl MissionDetail {
    /// Project this row onto the metadata the shared mission compiler reads.
    ///
    /// The one input the editor's export cannot derive from the document itself. Returning the
    /// struct rather than a hand-built JSON object means there is no key to mistype. Note that the
    /// `author` field carries the account id, not the display name — the display name travels
    /// beside it — which is the one field a reasonable reading gets wrong, and getting it wrong
    /// would produce a document that looks right and is not the one the server builds.
    pub fn compiled_meta(&self) -> mission_compiler::MissionMeta {
        mission_compiler::MissionMeta {
            id: self.id.as_str().into(),
            title: self.title.clone(),
            author: self.author_id.to_string(),
            terrain: self.terrain.clone(),
            custom_terrain_name: self.custom_terrain_name.clone().unwrap_or_default(),
            max_players: self.max_players,
            time_of_day: self.time_of_day.clone(),
            weather_preset: self.weather.clone(),
        }
    }
}

/// One item a mission's armoury offers.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmoryItem {
    /// The record's id.
    pub id: MissionArmoryId,
    /// Display name of the item.
    pub item_name: String,
    /// `null` = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,
    /// Every field the API sends beyond the ones named here, kept so a round trip loses nothing.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One faction's slice of a mission's armoury.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmoryFaction {
    /// The faction's name.
    pub faction: String,
    /// The items the faction's armory offers.
    pub items: Vec<ArmoryItem>,
    /// Every field the API sends beyond the ones named here, kept so a round trip loses nothing.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}
