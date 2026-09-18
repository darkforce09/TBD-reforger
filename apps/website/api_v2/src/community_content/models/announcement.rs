//! Announcement models: the news-feed / CMS row and its two Postgres ENUM vocabularies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::{go_time, go_time_opt};

/// Announcement statuses (Postgres ENUM `announcement_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "announcement_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementStatus {
    Draft,
    Published,
    Archived,
}

/// Announcement tags (Postgres ENUM `announcement_tag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "announcement_tag", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementTag {
    Update,
    Event,
    ModpackUpdate,
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
    pub id: Uuid,
    pub title: String,
    pub body: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub snippet: String,
    pub tag: AnnouncementTag,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub thumbnail_url: String,
    pub author_id: String,
    pub status: AnnouncementStatus,
    pub is_pinned: bool,
    pub pushed_to_discord: bool,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub discord_message_id: String,
    #[serde(with = "go_time_opt", skip_serializing_if = "Option::is_none", default)]
    pub published_at: Option<DateTime<Utc>>,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "go_time")]
    pub updated_at: DateTime<Utc>,
}
