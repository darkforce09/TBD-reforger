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

use serde::{Deserialize, Serialize};

/// One modpack: what it is called, and the mods it pins.
/// @contract modpack.schema.json#/definitions/Modpack
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Modpack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub total_size_bytes: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workshop_url: String,
    pub is_current: bool,
    pub created_at: String,
}

/// The modpack payload a single-modpack endpoint returns, and one row of the modpack list.
/// @contract modpack.schema.json#/definitions/Modpack
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ModpackDto {
    #[serde(flatten)]
    pub modpack: Modpack,
    /// The pack's mods, key dependencies first, then by `sort_order`.
    pub mods: Vec<ModpackMod>,
}

/// One mod of a modpack. `workshop_id` is the id the game launcher resolves, `mod_guid` the
/// addon's own identifier and `version` the pinned version; each is empty when unset.
/// @contract modpack.schema.json#/definitions/ModpackMod
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModpackMod {
    pub id: String,
    pub modpack_id: String,
    pub name: String,
    pub is_key_dependency: bool,
    pub sort_order: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workshop_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mod_guid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
}

/// One announcement, as the public feed, the dashboard and the content manager list it.
///
/// `tag` is one of `update`, `event`, `modpack_update` and `important`; `status` is `draft`,
/// `published` or `archived`. `author_id` is the author's Discord id.
/// @contract announcement.schema.json#/definitions/Announcement
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Announcement {
    pub id: String,
    pub title: String,
    pub body: String,
    /// The preview line the author wrote; empty when there is none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub snippet: String,
    pub tag: String,
    /// The hero image address; empty when the announcement has none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub thumbnail_url: String,
    pub author_id: String,
    pub status: String,
    pub is_pinned: bool,
    pub pushed_to_discord: bool,
    /// The chat message the announcement was pushed as; empty until it is pushed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub discord_message_id: String,
    /// When the announcement was first published; absent on a draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
