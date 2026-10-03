//! Announcement models: the news-feed / CMS row and its two Postgres ENUM vocabularies.
//!
//! @contract announcement.schema.json#/definitions/Announcement
//! @contract announcement.schema.json#/definitions/AnnouncementTag
//! @contract announcement.schema.json#/definitions/AnnouncementStatus

use api_identifiers::{AnnouncementId, DiscordMessageId, DiscordUserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// Announcement statuses (Postgres ENUM `announcement_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "announcement_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementStatus {
    /// Authored but not yet shown in the member feed.
    Draft,
    /// Shown in the member feed.
    Published,
    /// Withdrawn from the feed and kept for the record.
    Archived,
}

/// Announcement tags (Postgres ENUM `announcement_tag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "announcement_tag", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementTag {
    /// A general community update.
    Update,
    /// An event notice.
    Event,
    /// A change to the current modpack.
    ModpackUpdate,
    /// A notice every member is expected to read.
    Important,
}

impl AnnouncementTag {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            AnnouncementTag::Update => "update",
            AnnouncementTag::Event => "event",
            AnnouncementTag::ModpackUpdate => "modpack_update",
            AnnouncementTag::Important => "important",
        }
    }
}

/// News-feed / CMS announcement.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Announcement {
    /// The announcement's key.
    pub id: AnnouncementId,
    /// The headline.
    pub title: String,
    /// The full markdown body.
    pub body: String,
    /// The feed preview; omitted on the wire when empty.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub snippet: String,
    /// The category the feed files it under.
    pub tag: AnnouncementTag,
    /// The thumbnail image URL; omitted on the wire when empty.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub thumbnail_url: String,
    /// The Discord id of the administrator who wrote it.
    pub author_id: DiscordUserId,
    /// Where it is in its draft, published and archived life.
    pub status: AnnouncementStatus,
    /// Whether the feed shows it above the unpinned ones.
    pub is_pinned: bool,
    /// Whether the Discord webhook has posted it.
    pub pushed_to_discord: bool,
    /// The id of the webhook message that posted it; omitted on the wire when empty.
    #[serde(skip_serializing_if = "DiscordMessageId::is_empty", default)]
    pub discord_message_id: DiscordMessageId,
    /// When it was first published; absent while it is a draft.
    #[serde(
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub published_at: Option<DateTime<Utc>>,
    /// When the row was created.
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// When the row was last written.
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}
