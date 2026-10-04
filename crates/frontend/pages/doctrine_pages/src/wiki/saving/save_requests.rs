//! The bodies of the wiki's two writes: saving a draft, and restoring an older revision.
//!
//! **Role:** builds the [`WikiSaveRequest`] a draft save and a restore send to `PUT /wiki/{slug}`.
//! **Position:** called by the editor's save button and the revision view's restore action;
//! the save submission sends what it builds.
//! **Signals & state:** none; pure functions.
//! **Invariants:** every request names a numeric `base_revision` — the page never creates a
//! manual, so it never sends `null`. A draft save sends the revision the draft started from (the
//! article's own revision when there is no draft) and the article's stored category, title, icon
//! and navigation order; a restore sends every field of the old revision with the article's
//! current revision as its base, so a restore over a page that moved on is refused, never lost.

use super::super::page_state::WikiDraft;
use frontend_api_dtos::wiki::{WikiArticle, WikiRevision, WikiSaveRequest};

/// The save of `draft` over `article`; with no draft, the article's own body at its revision.
pub(in super::super) fn draft_save_request(
    article: &WikiArticle,
    draft: Option<&WikiDraft>,
) -> WikiSaveRequest {
    WikiSaveRequest {
        category: article.category.clone(),
        title: article.title.clone(),
        icon: article.icon.clone(),
        nav_order: article.nav_order,
        body_md: draft.map_or_else(|| article.body_md.clone(), |draft| draft.body_md.clone()),
        base_revision: Some(draft.map_or(article.revision, |draft| draft.base_revision)),
    }
}

/// The save that makes `revision` the page's content again, over the page's
/// `current_revision`.
pub(in super::super) fn restore_request(
    revision: &WikiRevision,
    current_revision: i64,
) -> WikiSaveRequest {
    WikiSaveRequest {
        category: revision.category.clone(),
        title: revision.title.clone(),
        icon: revision.icon.clone(),
        nav_order: revision.nav_order,
        body_md: revision.body_md.clone(),
        base_revision: Some(current_revision),
    }
}

#[cfg(test)]
#[path = "tests/save_requests.rs"]
mod tests;
