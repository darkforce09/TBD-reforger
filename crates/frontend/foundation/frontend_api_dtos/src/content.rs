//! Published content: the modpacks a deployment requires, and the announcements the unit posts.
//!
//! **Role:** the modpack listing and the single-modpack payload the content screens read, with
//! each pack's mod rows, and the announcement row the public feed, the dashboard and the content
//! manager read.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** field names are the wire contract of the backend's models. A text field the
//! backend skips when empty (`workshop_url`, a mod's `workshop_id`, `mod_guid` and `version`, an
//! announcement's `snippet`, `thumbnail_url` and `discord_message_id`) reads as an empty string
//! and is left out again when serialising; an announcement's `published_at` is absent until the
//! row is first published. Tags and statuses travel as strings, so a value the API adds renders
//! through the page's neutral fallback instead of failing the read.
//! @contract modpack.schema.json#/definitions/Modpack
//! @contract modpack.schema.json#/definitions/ModpackList
//! @contract announcement.schema.json#/definitions/Announcement
//! @contract announcement.schema.json#/definitions/AnnouncementPage

use super::identifiers::{
    AnnouncementId, DiscordMessageId, DiscordUserId, ModpackId, ModpackModId, WorkshopItemId,
    identifier_is_empty,
};
use serde::{Deserialize, Serialize};

/// One modpack: what it is called, and the mods it pins.
/// @contract modpack.schema.json#/definitions/Modpack
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Modpack {
    /// The modpack's key.
    pub id: ModpackId,
    /// The display name.
    pub name: String,
    /// The pack's own version label.
    pub version: String,
    /// The download size of every mod together, in bytes.
    pub total_size_bytes: i64,
    /// The Workshop collection URL; omitted on the wire when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workshop_url: String,
    /// Whether this is the one current pack members are told to run.
    pub is_current: bool,
    /// When the row was created.
    pub created_at: String,
}

/// The modpack payload a single-modpack endpoint returns, and one row of the modpack list.
/// @contract modpack.schema.json#/definitions/Modpack
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ModpackDto {
    /// The pack row, flattened into the answer.
    #[serde(flatten)]
    pub modpack: Modpack,
    /// The pack's mods, key dependencies first, then by `sort_order`.
    pub mods: Vec<ModpackMod>,
}

/// One mod of a modpack. `workshop_id` is the id the game launcher resolves, `mod_guid` the
/// addon's own identifier and `version` the pinned version; each is empty when unset.
/// @contract modpack.schema.json#/definitions/ModpackMod
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModpackMod {
    /// The mod row's key.
    pub id: ModpackModId,
    /// The modpack the mod belongs to.
    pub modpack_id: ModpackId,
    /// The display name.
    pub name: String,
    /// Whether the pack is unusable without it.
    pub is_key_dependency: bool,
    /// Its position in the pack's mod list, ascending.
    pub sort_order: i64,
    /// The Reforger Workshop item id (`modId`); omitted on the wire when empty.
    #[serde(default, skip_serializing_if = "identifier_is_empty")]
    pub workshop_id: WorkshopItemId,
    /// The mod's local GUID; omitted on the wire when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mod_guid: String,
    /// The pinned mod version; omitted on the wire when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
}

/// One announcement, as the public feed, the dashboard and the content manager list it.
///
/// `tag` is one of `update`, `event`, `modpack_update` and `important`; `status` is `draft`,
/// `published` or `archived`. `author_id` is the author's Discord id.
/// @contract announcement.schema.json#/definitions/Announcement
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Announcement {
    /// The announcement's key.
    pub id: AnnouncementId,
    /// The headline.
    pub title: String,
    /// The full markdown body.
    pub body: String,
    /// The preview line the author wrote; empty when there is none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub snippet: String,
    /// The category the feed files it under.
    pub tag: String,
    /// The hero image address; empty when the announcement has none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub thumbnail_url: String,
    /// The Discord id of the administrator who wrote it.
    pub author_id: DiscordUserId,
    /// Where it is in its draft, published and archived life.
    pub status: String,
    /// Whether the feed shows it above the unpinned ones.
    pub is_pinned: bool,
    /// Whether the Discord webhook has posted it.
    pub pushed_to_discord: bool,
    /// The chat message the announcement was pushed as; empty until it is pushed.
    #[serde(default, skip_serializing_if = "identifier_is_empty")]
    pub discord_message_id: DiscordMessageId,
    /// When the announcement was first published; absent on a draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    /// When the row was created.
    pub created_at: String,
    /// When the row was last written.
    pub updated_at: String,
}
