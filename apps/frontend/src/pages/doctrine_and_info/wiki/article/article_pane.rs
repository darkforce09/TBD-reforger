//! The detail half of the wiki: one manual fetched on its own, laid out with its history.
//!
//! **Role:** fetches the open manual from `GET /wiki/{slug}` and lays it out — the header, the
//! refusal notice, the article area and the revision panel — or shows its loading or failure
//! line.
//! **Position:** the detail pane of the wiki's split view, mounted afresh for each manual.
//! **Signals & state:** creates the manual's [`ArticleState`], the article resource (refetched by
//! its reload counter) and the two history resources.
//! **Invariants:** a reload keeps the previous article on screen until the new one arrives, so a
//! save never blanks the pane. The module compiles for `wasm32` only, the one target that
//! mounts the wiki.

use super::super::display_text::load_failure_text;
use super::super::page_state::WikiPageState;
use super::super::revisions::{
    revision_list, revision_list_resource, revision_resource, RevisionListResource,
    RevisionResource,
};
use super::super::saving::save_problem_view;
use super::article_body::article_body;
use super::article_header::article_header;
use super::ArticleState;
use crate::foundation::auth::AuthStore;
use crate::foundation::transport::client::Fetched;
use crate::foundation::transport::dto::wiki::WikiArticle;
use leptos::prelude::*;

/// The aside that holds the revision panel.
const HISTORY_ASIDE_CLASS: &str = "custom-scrollbar max-h-72 shrink-0 overflow-y-auto border-t border-white/10 xl:max-h-none xl:w-72 xl:border-t-0 xl:border-l";

/// `GET /wiki/{slug}` as a settled fetch.
async fn fetch_article(store: AuthStore, slug: String) -> Fetched<WikiArticle> {
    let path = super::super::api_paths::article_path(&slug);
    crate::foundation::transport::client::api_get::<WikiArticle>(store, &path)
        .await
        .into()
}

/// The pane of the manual `slug`.
pub(in super::super) fn article_pane(page: WikiPageState, slug: String) -> impl IntoView {
    let article_state = ArticleState::new(slug);
    let store = page.store;
    let article = LocalResource::new(move || {
        article_state.article_generation.track();
        fetch_article(store, article_state.slug.get_value())
    });
    let revisions = revision_list_resource(store, article_state);
    let revision = revision_resource(store, article_state);
    move || match article.get() {
        None => view! {
            <section class="flex h-full items-center justify-center p-8">
                <p class="font-mono text-sm text-on-surface-variant">"Loading…"</p>
            </section>
        }
        .into_any(),
        Some(Fetched::Failed(failure)) => view! {
            <section class="flex h-full items-center justify-center p-8">
                <p class="font-mono text-sm text-error-alert">
                    {load_failure_text(&failure, "this manual")}
                </p>
            </section>
        }
        .into_any(),
        Some(Fetched::Data(loaded)) => {
            loaded_article(page, article_state, loaded, revisions, revision).into_any()
        }
    }
}

/// The layout of a fetched manual.
fn loaded_article(
    page: WikiPageState,
    article_state: ArticleState,
    loaded: WikiArticle,
    revisions: RevisionListResource,
    revision: RevisionResource,
) -> impl IntoView {
    let current_revision = loaded.revision;
    let article = StoredValue::new(loaded);
    view! {
        <section class="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
            {article_header(page, article_state, article)}
            {save_problem_view(page, article_state)}
            <div class="flex min-h-0 flex-1 flex-col xl:flex-row">
                {article_body(page, article_state, article, revision)}
                <aside class=HISTORY_ASIDE_CLASS>
                    {revision_list(article_state, revisions, current_revision)}
                </aside>
            </div>
        </section>
    }
}
