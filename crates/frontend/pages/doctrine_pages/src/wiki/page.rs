//! The doctrine wiki route: fetch the page list, pick a manual, and lay the two panes out.
//!
//! **Role:** owns the request for `GET /wiki` (the page summaries), resolves the route slug to a
//! manual, builds the state the panes share, and arranges the index and the open manual in a
//! split view.
//! **Position:** the `/wiki` and `/wiki/:slug` routes, behind the authentication gate.
//! **Signals & state:** a `LocalResource` for the page list; the shared `WikiPageState` (the
//! `search`, `mode` and `drafts` signals and an `is_admin` memo over the `AuthStore` from
//! context); the route's `:slug` parameter via the router.
//! **Invariants:** the admin memo re-reads the store, so the editing affordances appear only for
//! a signed-in administrator and never during bootstrap. Opening another manual mounts a fresh
//! article pane and returns the mode to reading. The route, its fetch and its board compile for
//! `wasm32` only; the slug resolution is pure and tested natively.

#[cfg(target_arch = "wasm32")]
use super::article::article_pane;
#[cfg(target_arch = "wasm32")]
use super::category_nav::{manual_index, master_header};
#[cfg(target_arch = "wasm32")]
use super::page_state::{PageListResource, WikiMode, WikiPageState};
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::DataEnvelope;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::role::{Role, has_min_role_authed};
use frontend_api_dtos::wiki::WikiPageSummary;
#[cfg(target_arch = "wasm32")]
use frontend_transport::client::Fetched;
#[cfg(target_arch = "wasm32")]
use frontend_ui::split_pane::GlassSplit;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

#[cfg(test)]
#[path = "tests/wiki.rs"]
mod tests;

/// The slug to open, given the page list and the route parameter.
///
/// A parameter that names a manual in the list wins; anything else falls back to the first row,
/// which the API orders first by nav order and then by title. `None` means the list is empty.
fn resolve_slug(pages: &[WikiPageSummary], slug: Option<&str>) -> Option<String> {
    if let Some(wanted) = slug
        && pages.iter().any(|page| page.slug == wanted)
    {
        return Some(wanted.to_string());
    }
    pages
        .first()
        .map(|page| page.slug.clone())
        .filter(|slug| !slug.is_empty())
}

/// `GET /wiki` as a settled fetch.
#[cfg(target_arch = "wasm32")]
async fn fetch_page_list(
    store: frontend_session::AuthStore,
) -> Fetched<DataEnvelope<WikiPageSummary>> {
    frontend_transport::client::api_get::<DataEnvelope<WikiPageSummary>>(
        store,
        super::api_paths::PAGE_LIST_PATH,
    )
    .await
    .into()
}

/// The doctrine wiki, behind the authentication gate.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn WikiPage() -> impl IntoView {
    view! {
        <frontend_session::AuthGate>
            <WikiInner />
        </frontend_session::AuthGate>
    }
}

/// Fetches the page list and renders the board, a loading line, or a failure line.
#[cfg(target_arch = "wasm32")]
#[component]
fn WikiInner() -> impl IntoView {
    let store = expect_context::<frontend_session::AuthStore>();
    let page_list: PageListResource = LocalResource::new(move || fetch_page_list(store));
    // Re-read the store on every change: the browse-mode role check treats a signed-out
    // visitor as permitted, so it must never drive the edit and save affordances.
    let is_admin =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin));
    let page = WikiPageState {
        store,
        is_admin,
        search: RwSignal::new(String::new()),
        mode: RwSignal::new(WikiMode::Read),
        drafts: RwSignal::new(std::collections::HashMap::new()),
        page_list,
    };
    // Whether the list has loaded (and, if so, whether it arrived): a refetch that succeeds again
    // leaves this unchanged, so the board and the open manual stay mounted across a save.
    let settled = Memo::new(move |_| page_list.get().map(|fetched| fetched.data().is_some()));
    move || match settled.get() {
        None => view! { <p class="text-on-surface-variant">"Loading…"</p> }.into_any(),
        Some(true) => wiki_board(page).into_any(),
        Some(false) => view! { <p class="text-error">"Failed to load wiki."</p> }.into_any(),
    }
}

/// The split view over the fetched page list, which it re-reads whenever the list refetches.
#[cfg(target_arch = "wasm32")]
fn wiki_board(page: WikiPageState) -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let summaries = Memo::new(move |_| {
        page.page_list.with(|fetched| {
            fetched
                .as_ref()
                .and_then(|fetched| fetched.data())
                .map(|envelope| envelope.data.clone())
                .unwrap_or_default()
        })
    });
    let selected = Memo::new(move |_| {
        summaries.with(|list| resolve_slug(list, params.read().get("slug").as_deref()))
    });

    Effect::new(move |previous: Option<Option<String>>| {
        let slug = selected.get();
        if previous.as_ref().is_some_and(|previous| previous != &slug) {
            page.mode.set(WikiMode::Read);
        }
        slug
    });

    view! {
        <GlassSplit
            master_width="17rem"
            master_header={master_header(page.search).into_any()}
            master={view! {
                {move || {
                    let query = page.search.get();
                    summaries.with(|list| manual_index(selected.get(), &query, list))
                }}
            }
                .into_any()}
            detail={view! {
                {move || match selected.get() {
                    Some(slug) => article_pane(page, slug).into_any(),
                    None => view! {
                        <section class="flex h-full items-center justify-center p-8">
                            <p class="font-mono text-sm text-on-surface-variant">
                                {if summaries.with(Vec::is_empty) {
                                    "No manuals yet."
                                } else {
                                    "Select a manual."
                                }}
                            </p>
                        </section>
                    }
                    .into_any(),
                }}
            }
                .into_any()}
        />
    }
}
