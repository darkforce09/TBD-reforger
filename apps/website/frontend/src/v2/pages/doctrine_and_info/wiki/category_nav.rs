//! The master half of the wiki: the search box and the category-grouped list of manuals.
//!
//! **Role:** renders the doctrine index — one labelled group per category, each holding the
//! manuals that survived the search box, as links that navigate to `/wiki/<slug>`.
//! **Position:** the master pane of the wiki's split view, above which its own header sits.
//! **Signals & state:** reads the search `RwSignal<String>` the page owns and writes it through
//! `SidebarSearch`; navigation is the router's, so no selection state lives here.
//! **Invariants:** groups appear in the order the API returned their first member, which is the
//! nav order the wiki is authored in; a group whose manuals all filtered out is not rendered.

use super::helpers::vstr;
use crate::v2::core::ui::split_pane::{ListDetailItem, SidebarSearch};
use leptos::prelude::*;
use serde_json::Value;

/// The distinct categories of `pages`, in the order each was first seen.
///
/// The API orders the list by nav order and then title, so first-seen order is authoring order.
/// Rows with no category are skipped.
pub(super) fn category_order(pages: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for p in pages {
        let c = vstr(p, "category");
        if !c.is_empty() && !out.iter().any(|x| x == &c) {
            out.push(c);
        }
    }
    out
}

/// The index header: the section label and the search box bound to `search`.
pub(super) fn master_header(search: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <p class="font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
                "SOPs & Manuals"
            </p>
            <SidebarSearch placeholder="Search manuals..." bind=search />
        </div>
    }
}

/// The grouped list of manuals.
///
/// `active_slug` is the manual currently open, `query` the live search text, and `pages` the
/// full list. A row matches when the query matches its title or its category; clicking one
/// navigates to that manual's route rather than mutating local state.
pub(super) fn manual_index(
    active_slug: Option<String>,
    query: &str,
    pages: &[Value],
) -> impl IntoView {
    let query = query.to_string();
    let active = active_slug.unwrap_or_default();
    category_order(pages)
        .into_iter()
        .filter_map(move |category| {
            let rows: Vec<Value> = pages
                .iter()
                .filter(|p| vstr(p, "category") == category)
                .filter(|p| {
                    crate::v2::core::ui::split_pane::search_matches(
                        &query,
                        &format!("{} {}", vstr(p, "title"), vstr(p, "category")),
                    )
                })
                .cloned()
                .collect();
            if rows.is_empty() {
                return None;
            }
            let cat_label = category.clone();
            Some(view! {
                <div class="mb-3">
                    <p class="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                        {cat_label}
                    </p>
                    <div class="mt-1 flex flex-col gap-1">
                        {rows
                            .into_iter()
                            .map(|m| {
                                let id = vstr(&m, "slug");
                                let title = vstr(&m, "title");
                                let navigate = leptos_router::hooks::use_navigate();
                                let active_row = id == active;
                                view! {
                                    <ListDetailItem
                                        active=active_row
                                        title=view! { {title} }.into_any()
                                        on_click=Callback::new(move |()| {
                                            navigate(&format!("/wiki/{id}"), Default::default());
                                        })
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            })
        })
        .collect_view()
}
