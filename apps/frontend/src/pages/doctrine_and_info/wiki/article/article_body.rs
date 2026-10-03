//! The body of the open manual: the rendered blocks, the markdown editor, or an older revision.
//!
//! **Role:** chooses what fills the article area — the revision on view when there is one,
//! otherwise the raw-markdown editor for an administrator in edit mode, otherwise the manual's
//! blocks — and keeps each manual's draft as the administrator types.
//! **Position:** the scrolling area of the article pane, beside the revision panel.
//! **Signals & state:** reads the article's `viewing`, and the page's `mode`, `is_admin` and
//! `drafts`; the editor writes `drafts`.
//! **Invariants:** the blocks are the server's parse of the saved body, so an unsaved draft is
//! never rendered as if saved: reading a manual with a draft shows the saved text under a notice
//! that a draft exists. The editor reads the drafts untracked, so typing never re-renders it; a
//! draft keeps the revision its edit started from.

use super::super::blocks::render_blocks;
use super::super::page_state::{updated_draft, WikiMode, WikiPageState};
use super::super::revisions::{revision_view, RevisionResource};
use super::ArticleState;
use crate::foundation::transport::dto::wiki::WikiArticle;
use leptos::prelude::*;

/// The scrolling frame of the rendered text.
const READING_FRAME_CLASS: &str = "custom-scrollbar min-h-0 flex-1 overflow-y-auto p-8 md:p-12";
/// The editor.
const EDITOR_CLASS: &str = "min-h-0 w-full flex-1 resize-none border-none bg-transparent p-8 font-mono text-sm leading-relaxed text-on-surface-variant outline-none focus:ring-0 md:p-12";
/// The notice over a manual that has an unsaved draft.
const DRAFT_NOTICE_CLASS: &str = "mb-6 rounded-xl border border-tactical-yellow/40 bg-tactical-yellow/10 px-4 py-3 text-sm text-on-surface-variant";

/// The article area of `article`.
pub(super) fn article_body(
    page: WikiPageState,
    article_state: ArticleState,
    article: StoredValue<WikiArticle>,
    revision: RevisionResource,
) -> impl IntoView {
    move || {
        if article_state.viewing.get().is_some() {
            let current_revision = article.with_value(|article| article.revision);
            return view! {
                <article class=READING_FRAME_CLASS>
                    <div class="max-w-3xl">
                        {revision_view(page, article_state, revision, current_revision)}
                    </div>
                </article>
            }
            .into_any();
        }
        if page.mode.get() == WikiMode::Edit && page.is_admin.get() {
            return editor(page, article_state, article).into_any();
        }
        let blocks = article.with_value(|article| render_blocks(&article.blocks));
        view! {
            <article class=READING_FRAME_CLASS>
                <div class="max-w-3xl">
                    {draft_notice(page, article_state, article)} {blocks}
                </div>
            </article>
        }
        .into_any()
    }
}

/// The notice shown over a manual that has an unsaved draft, or nothing.
fn draft_notice(
    page: WikiPageState,
    article_state: ArticleState,
    article: StoredValue<WikiArticle>,
) -> impl IntoView {
    move || {
        let slug = article_state.slug.get_value();
        let base = page
            .drafts
            .with(|drafts| drafts.get(&slug).map(|draft| draft.base_revision))?;
        let saved = article.with_value(|article| article.revision);
        Some(view! {
            <p role="status" class=DRAFT_NOTICE_CLASS>
                {format!(
                    "You have an unsaved draft of this manual, started from revision {base}. \
                     Below is the saved revision {saved}; switch to [ EDIT ] to continue the draft \
                     or save it.",
                )}
            </p>
        })
    }
}

/// The raw-markdown editor: the draft when there is one, otherwise the saved body.
fn editor(
    page: WikiPageState,
    article_state: ArticleState,
    article: StoredValue<WikiArticle>,
) -> impl IntoView {
    let slug = article_state.slug.get_value();
    let (saved_body, revision) =
        article.with_value(|article| (article.body_md.clone(), article.revision));
    let initial = page
        .drafts
        .with_untracked(|drafts| drafts.get(&slug).map(|draft| draft.body_md.clone()))
        .unwrap_or(saved_body);
    view! {
        <textarea
            prop:value=initial
            spellcheck="false"
            aria-label="Manual markdown"
            on:input=move |ev| {
                let body = event_target_value(&ev);
                page.drafts
                    .update(|drafts| {
                        let next = updated_draft(drafts.get(&slug), revision, body);
                        drafts.insert(slug.clone(), next);
                    });
            }
            class=EDITOR_CLASS
        ></textarea>
    }
}
