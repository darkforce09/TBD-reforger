//! The doctrine wiki shapes: the stored page and revision rows, and every body the wiki routes
//! read or answer.
//!
//! **Role:** the rows of `wiki_pages` ([`WikiPage`]) and `wiki_page_revisions`
//! ([`WikiPageRevision`]), and the wire shapes of the wiki routes: the navigation list, the
//! article, the administrator's save and its refusal, and the revision history.
//! **Position:** filled by the queries of
//! [`crate::community_content::handlers::wiki_knowledgebase`]; the `blocks` of an article or a
//! revision come from [`crate::community_content::services::wiki_markup::read_markup`]; the
//! doctrine wiki page of the single-page app reads the answers.
//! **Signals & state:** none; plain data.
//! **Invariants:** `icon`, `updated_by` and `author_id` are omitted on the wire when empty (the
//! queries `COALESCE` a stored null icon or author to `''`); timestamps are RFC 3339 UTC; the
//! save body names every field and refuses any other; the caller's identity never comes from a
//! body.
//! @contract wiki-page.schema.json#/definitions/WikiArticle

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::community_content::services::wiki_markup::{WikiBlock, WikiMarkupFinding};
use crate::core::wire_format::rfc3339_utc;

/// One stored `wiki_pages` row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WikiPage {
    pub id: Uuid,
    pub slug: String,
    pub category: String,
    pub title: String,
    /// `''` when the page has no icon.
    pub icon: String,
    pub body_md: String,
    pub nav_order: i64,
    /// The number of the page's current revision.
    pub revision: i32,
    /// The Discord id of the last editor, when known.
    pub updated_by: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// One page in the doctrine navigation.
/// @contract wiki-page.schema.json#/definitions/WikiPageSummary
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WikiPageSummary {
    pub slug: String,
    pub category: String,
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
    pub nav_order: i64,
    pub revision: i32,
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// `GET /api/v1/wiki`: every page's summary.
/// @contract wiki-page.schema.json#/definitions/WikiPageList
#[derive(Debug, Clone, Serialize)]
pub struct WikiPageList {
    pub data: Vec<WikiPageSummary>,
}

/// One page: its summary fields, its markdown and the blocks parsed from it.
/// @contract wiki-page.schema.json#/definitions/WikiArticle
#[derive(Debug, Clone, Serialize)]
pub struct WikiArticle {
    pub id: Uuid,
    pub slug: String,
    pub category: String,
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
    pub nav_order: i64,
    pub revision: i32,
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    pub body_md: String,
    pub blocks: Vec<WikiBlock>,
}

impl WikiArticle {
    /// The article of `page`, whose markdown reads to `blocks`; an empty editor id counts as
    /// unknown.
    pub fn new(page: WikiPage, blocks: Vec<WikiBlock>) -> Self {
        Self {
            id: page.id,
            slug: page.slug,
            category: page.category,
            title: page.title,
            icon: page.icon,
            nav_order: page.nav_order,
            revision: page.revision,
            updated_at: page.updated_at,
            updated_by: page.updated_by.filter(|editor| !editor.is_empty()),
            body_md: page.body_md,
            blocks,
        }
    }
}

/// `PUT /api/v1/wiki/{slug}` body. Every field is required, `base_revision` included (`null`
/// creates the page), and an unknown field is refused.
/// @contract wiki-page.schema.json#/definitions/WikiSaveRequest
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiSaveRequest {
    pub category: String,
    pub title: String,
    /// `''` for a page without an icon.
    pub icon: String,
    /// The navigation position; `0` is a real position, so the field has no default.
    pub nav_order: i64,
    pub body_md: String,
    /// The revision the edit starts from, or `null` to create the page.
    #[serde(deserialize_with = "Option::deserialize")]
    pub base_revision: Option<i64>,
}

/// The `details.code` of a refused wiki save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiSaveRefusalCode {
    /// `409`: the page's current revision is not the save's base.
    WikiRevisionConflict,
    /// `400`: `body_md` is over the size limit.
    WikiBodyTooLarge,
    /// `422`: the markup holds refused constructs.
    WikiMarkupRefused,
}

/// The `details` of a refused wiki save.
/// @contract wiki-page.schema.json#/definitions/WikiSaveRefusal
#[derive(Debug, Clone, Serialize)]
pub struct WikiSaveRefusal {
    pub code: WikiSaveRefusalCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub findings: Option<Vec<WikiMarkupFinding>>,
}

/// One stored `wiki_page_revisions` row, with the slug it was saved under.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WikiPageRevision {
    pub slug: String,
    pub revision: i32,
    pub category: String,
    pub title: String,
    /// `''` when the revision has no icon.
    pub icon: String,
    pub nav_order: i64,
    pub body_md: String,
    /// `''` when the editor is unknown.
    pub author_id: String,
    pub created_at: DateTime<Utc>,
}

/// One entry of a page's revision history.
/// @contract wiki-page.schema.json#/definitions/WikiRevisionSummary
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WikiRevisionSummary {
    pub revision: i32,
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author_id: String,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

/// `GET /api/v1/wiki/{slug}/revisions`: one page of the history, newest first.
/// @contract wiki-page.schema.json#/definitions/WikiRevisionPage
#[derive(Debug, Clone, Serialize)]
pub struct WikiRevisionPage {
    pub items: Vec<WikiRevisionSummary>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

/// `GET /api/v1/wiki/{slug}/revisions/{revision}`: the page as one revision saved it.
/// @contract wiki-page.schema.json#/definitions/WikiRevision
#[derive(Debug, Clone, Serialize)]
pub struct WikiRevision {
    pub slug: String,
    pub revision: i32,
    pub category: String,
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
    pub nav_order: i64,
    pub body_md: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author_id: String,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    pub blocks: Vec<WikiBlock>,
}

impl WikiRevision {
    /// The revision `row`, whose markdown reads to `blocks`.
    pub fn new(row: WikiPageRevision, blocks: Vec<WikiBlock>) -> Self {
        Self {
            slug: row.slug,
            revision: row.revision,
            category: row.category,
            title: row.title,
            icon: row.icon,
            nav_order: row.nav_order,
            body_md: row.body_md,
            author_id: row.author_id,
            created_at: row.created_at,
            blocks,
        }
    }
}
