//! The doctrine wiki route: fetch the manual list, pick one, and lay the two panes out.
//!
//! **Role:** owns the request for `GET /wiki`, resolves the route slug to a manual, holds the
//! read/edit mode and the unsaved drafts, and arranges the index and the article in a split view.
//! **Position:** the `/wiki` and `/wiki/:slug` routes, behind the authentication gate.
//! **Signals & state:** a `LocalResource` for the manual list; `search`, `mode`, `drafts`,
//! `save_busy` and `save_err` signals shared with the two panes; an `is_admin` memo over the
//! `AuthStore` from context; the route's `:slug` parameter via the router.
//! **Invariants:** the admin memo re-reads the store, so the editing affordances appear only for
//! a signed-in administrator and never during bootstrap. Selecting another manual resets the
//! mode to reading and clears the save error. The fetch runs on `wasm32` only; natively the
//! resource resolves to `None` and the page renders its failure text.

use super::category_nav::{manual_index, master_header};
use super::helpers::vstr;
use super::markdown_article::article;
use crate::v2::core::api::dto::DataEnvelope;
use crate::v2::core::auth::{has_min_role_authed, Role};
use crate::v2::core::ui::split_pane::GlassSplit;
use leptos::prelude::*;
use serde_json::Value;

#[cfg(test)]
#[path = "tests/wiki.rs"]
mod tests;

/// The slug to open, given the fetched list and the route parameter.
///
/// A parameter that names a manual in the list wins; anything else falls back to the first row,
/// which the API orders first by nav order and then by title. `None` means the list is empty.
fn resolve_slug(pages: &[Value], slug: Option<&str>) -> Option<String> {
    if let Some(s) = slug {
        if pages.iter().any(|p| vstr(p, "slug") == s) {
            return Some(s.to_string());
        }
    }
    pages
        .first()
        .map(|p| vstr(p, "slug"))
        .filter(|s| !s.is_empty())
}

/// Which half of the article surface the detail pane shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WikiMode {
    Read,
    Edit,
}

/// The doctrine wiki, behind the authentication gate.
#[component]
pub fn WikiPage() -> impl IntoView {
    view! {
        <crate::v2::core::ui::AuthGate>
            <WikiInner />
        </crate::v2::core::ui::AuthGate>
    }
}

/// Fetches the manual list and renders the board, a loading line, or a failure line.
#[component]
fn WikiInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let pages = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<DataEnvelope<Value>>(store, "/wiki")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<Value>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                pages
                    .get()
                    .map(|opt| match opt {
                        Some(env) => wiki_board(env.data, pages).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load wiki."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// The split view over a fetched manual list.
///
/// `page_list` is that list and `pages_res` the resource it came from, kept so a save can
/// refetch it.
fn wiki_board(
    page_list: Vec<Value>,
    pages_res: LocalResource<Option<DataEnvelope<Value>>>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    // Re-read the store on every change: the browse-mode role check treats a signed-out
    // visitor as permitted, so it must never drive the edit and save affordances.
    let is_admin =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin));
    let params = leptos_router::hooks::use_params_map();
    let search = RwSignal::new(String::new());
    let mode = RwSignal::new(WikiMode::Read);
    // The unsaved body of each manual, keyed by slug; a successful save drops the entry.
    let drafts = RwSignal::new(std::collections::HashMap::<String, String>::new());
    let save_busy = RwSignal::new(false);
    let save_err = RwSignal::new(None::<String>);

    let page_list_for_sel = page_list.clone();
    let selected =
        Memo::new(move |_| resolve_slug(&page_list_for_sel, params.read().get("slug").as_deref()));

    Effect::new(move |prev: Option<Option<String>>| {
        let id = selected.get();
        if prev.as_ref().is_some_and(|p| p != &id) {
            mode.set(WikiMode::Read);
            save_err.set(None);
        }
        id
    });

    let page_list_master = page_list.clone();
    let page_list_detail = page_list;

    view! {
        <GlassSplit
            master_width="17rem"
            master_header=master_header(search).into_any()
            master=view! {
                {move || {
                    manual_index(
                        selected.get(),
                        &search.get(),
                        &page_list_master,
                    )
                }}
            }
                .into_any()
            detail=view! {
                {move || {
                    let slug = selected.get();
                    let page = slug.as_ref().and_then(|s| {
                        page_list_detail.iter().find(|p| vstr(p, "slug") == *s).cloned()
                    });
                    match page {
                        Some(p) => article(
                            p,
                            mode,
                            drafts,
                            save_busy,
                            save_err,
                            is_admin.get(),
                            store,
                            pages_res,
                        )
                        .into_any(),
                        None => view! {
                            <section class="flex h-full items-center justify-center p-8">
                                <p class="font-mono text-sm text-on-surface-variant">
                                    {if page_list_detail.is_empty() {
                                        "No manuals yet."
                                    } else {
                                        "Select a manual."
                                    }}
                                </p>
                            </section>
                        }
                        .into_any(),
                    }
                }}
            }
                .into_any()
        />
    }
}
