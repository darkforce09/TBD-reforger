//! The wiki's SQL: the page and revision reads, and the writes of one save.
//!
//! **Role:** every statement the wiki handlers run against `wiki_pages` and
//! `wiki_page_revisions`, each over any Postgres executor so a save can run them inside its
//! transaction.
//! **Position:** called by the read, revision and save handlers of this folder; answers
//! [`WikiPage`], [`WikiPageSummary`], [`WikiRevisionSummary`] and [`WikiPageRevision`] rows.
//! **Signals & state:** none; one statement per call.
//! **Invariants:** a nullable column is read through `COALESCE` or into an `Option`; an empty
//! icon is stored as null; a revision row is copied from the page row it records, inside the
//! transaction that wrote the page, so the two never disagree; the summaries order by
//! `nav_order`, then `title`, then `slug`, and the history by revision, newest first.

use api_identifiers::{DiscordUserId, WikiPageId};
use sqlx::PgExecutor;

use crate::models::wiki::{
    WikiPage, WikiPageRevision, WikiPageSummary, WikiRevisionSummary, WikiSaveRequest,
};

/// Every page's summary in navigation order.
pub(super) async fn page_summaries<'e>(
    executor: impl PgExecutor<'e>,
) -> sqlx::Result<Vec<WikiPageSummary>> {
    sqlx::query_as(
        "SELECT slug, category, title, COALESCE(icon, '') AS icon, nav_order, revision, \
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
         FROM wiki_pages ORDER BY nav_order ASC, title ASC, slug ASC",
    )
    .fetch_all(executor)
    .await
}

/// The page stored under `slug`, if any.
pub(super) async fn page_by_slug<'e>(
    executor: impl PgExecutor<'e>,
    slug: &str,
) -> sqlx::Result<Option<WikiPage>> {
    sqlx::query_as(
        "SELECT id, slug, category, title, COALESCE(icon, '') AS icon, body_md, nav_order, \
         revision, updated_by, \
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
         FROM wiki_pages WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(executor)
    .await
}

/// The id and current revision of the page under `slug`, row-locked until the transaction
/// ends; `None` when no page has the slug.
pub(super) async fn lock_page<'e>(
    executor: impl PgExecutor<'e>,
    slug: &str,
) -> sqlx::Result<Option<(WikiPageId, i32)>> {
    sqlx::query_as("SELECT id, revision FROM wiki_pages WHERE slug = $1 FOR UPDATE")
        .bind(slug)
        .fetch_optional(executor)
        .await
}

/// The current revision of the page under `slug`, if any.
pub(super) async fn current_revision<'e>(
    executor: impl PgExecutor<'e>,
    slug: &str,
) -> sqlx::Result<Option<i32>> {
    sqlx::query_scalar("SELECT revision FROM wiki_pages WHERE slug = $1")
        .bind(slug)
        .fetch_optional(executor)
        .await
}

/// Inserts the page under `slug` at revision 1, edited by `editor`; `None` when a page with the
/// slug already exists, which is left untouched.
pub(super) async fn insert_page<'e>(
    executor: impl PgExecutor<'e>,
    slug: &str,
    request: &WikiSaveRequest,
    editor: &DiscordUserId,
) -> sqlx::Result<Option<WikiPageId>> {
    sqlx::query_scalar(
        "INSERT INTO wiki_pages \
         (slug, category, title, icon, body_md, nav_order, revision, updated_by, updated_at) \
         VALUES ($1, $2, $3, NULLIF($4, ''), $5, $6, 1, $7, now()) \
         ON CONFLICT (slug) DO NOTHING RETURNING id",
    )
    .bind(slug)
    .bind(&request.category)
    .bind(&request.title)
    .bind(&request.icon)
    .bind(&request.body_md)
    .bind(request.nav_order)
    .bind(editor)
    .fetch_optional(executor)
    .await
}

/// Replaces the content of page `page_id` with `request`, edited by `editor`, and answers its
/// new revision number.
pub(super) async fn update_page<'e>(
    executor: impl PgExecutor<'e>,
    page_id: WikiPageId,
    request: &WikiSaveRequest,
    editor: &DiscordUserId,
) -> sqlx::Result<i32> {
    sqlx::query_scalar(
        "UPDATE wiki_pages SET category = $2, title = $3, icon = NULLIF($4, ''), body_md = $5, \
         nav_order = $6, revision = revision + 1, updated_by = $7, updated_at = now() \
         WHERE id = $1 RETURNING revision",
    )
    .bind(page_id)
    .bind(&request.category)
    .bind(&request.title)
    .bind(&request.icon)
    .bind(&request.body_md)
    .bind(request.nav_order)
    .bind(editor)
    .fetch_one(executor)
    .await
}

/// Records the current content of page `page_id` as its revision row: the page's revision
/// number, fields, editor and update time.
pub(super) async fn record_current_revision<'e>(
    executor: impl PgExecutor<'e>,
    page_id: WikiPageId,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO wiki_page_revisions \
         (page_id, revision, slug, category, title, icon, nav_order, body_md, author_id, \
         created_at) \
         SELECT id, revision, slug, category, title, icon, nav_order, body_md, updated_by, \
         updated_at \
         FROM wiki_pages WHERE id = $1",
    )
    .bind(page_id)
    .execute(executor)
    .await?;
    Ok(())
}

/// The id of the page under `slug`, if any.
pub(super) async fn page_id<'e>(
    executor: impl PgExecutor<'e>,
    slug: &str,
) -> sqlx::Result<Option<WikiPageId>> {
    sqlx::query_scalar("SELECT id FROM wiki_pages WHERE slug = $1")
        .bind(slug)
        .fetch_optional(executor)
        .await
}

/// The number of revisions page `page_id` holds.
pub(super) async fn revision_count<'e>(
    executor: impl PgExecutor<'e>,
    page_id: WikiPageId,
) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM wiki_page_revisions WHERE page_id = $1")
        .bind(page_id)
        .fetch_one(executor)
        .await
}

/// `limit` revisions of page `page_id` from `offset`, newest first.
pub(super) async fn revision_summaries<'e>(
    executor: impl PgExecutor<'e>,
    page_id: WikiPageId,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<WikiRevisionSummary>> {
    sqlx::query_as(
        "SELECT revision, title, COALESCE(author_id, '') AS author_id, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at \
         FROM wiki_page_revisions WHERE page_id = $1 \
         ORDER BY revision DESC LIMIT $2 OFFSET $3",
    )
    .bind(page_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(executor)
    .await
}

/// Revision `revision` of the page under `slug`, if both exist.
pub(super) async fn revision_by_number<'e>(
    executor: impl PgExecutor<'e>,
    slug: &str,
    revision: i32,
) -> sqlx::Result<Option<WikiPageRevision>> {
    sqlx::query_as(
        "SELECT r.slug, r.revision, r.category, r.title, COALESCE(r.icon, '') AS icon, \
         r.nav_order, r.body_md, COALESCE(r.author_id, '') AS author_id, \
         COALESCE(r.created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at \
         FROM wiki_page_revisions r JOIN wiki_pages p ON p.id = r.page_id \
         WHERE p.slug = $1 AND r.revision = $2",
    )
    .bind(slug)
    .bind(revision)
    .fetch_optional(executor)
    .await
}
