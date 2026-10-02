//! The personnel route: one page of the roster on the left, one member's dossier on the right.
//!
//! **Role:** keeps the roster's address (page, page size and search) in the URL, fetches that page
//! of the roster, puts it behind the administrator gate, owns the search box and the sort and
//! filter controls, and arranges the table and its pager beside the dossier.
//! **Position:** the `/admin/personnel` route, rendered inside the navigation frame.
//! **Signals & state:** the address is a memo over the URL query (`use_query_map`), and every
//! control changes it by navigating in place (`use_navigate`, replacing the history entry). Also
//! owns `selected_id` (whose dossier is open), `sort_mode` and `filter_mode` (client-side, over
//! the loaded page), and `sync_busy` around the role resync. The served page lives in a
//! `LocalResource` keyed on the address's request path and read by both panes and the pager.
//! **Invariants:** the address re-runs the fetch, but sorting and filtering do not — the roster
//! route pages and searches, so those two reorder only the page already loaded. A new search or
//! page size starts at the first page, and a page served past the end sends the address back to
//! the first page. Both panes read the same fetched page, so the dossier can never show a member
//! the table is no longer listing. The request future is not `Send`, so a native build resolves it
//! to nothing and renders the failure branch.
#![allow(dead_code)]

use super::dossier::dossier;
use super::member_roster::{
    apply_roster_filter, apply_roster_sort, roster_table, FilterMode, SortMode,
};
use super::roster_pager::{roster_pager, PagerPosition};
use super::roster_query::RosterQuery;
use crate::v2::core::api::dto::administration::PersonnelPage;
use crate::v2::core::ui::{AdminGate, MaterialIcon};
use leptos::prelude::*;
use leptos_router::NavigateOptions;

/// The admin route that re-applies the Discord role mappings across the roster.
pub(super) const ADMIN_ROLES_SYNC_PATH: &str = "/admin/roles/sync";

/// Read the number of updated members out of a role-sync response.
///
/// A missing or non-integer count is an error, so a success status over an empty body cannot be
/// reported as a completed sync.
pub(super) fn roles_sync_updated_count(body: &serde_json::Value) -> Result<i64, &'static str> {
    body.get("updated")
        .and_then(|v| v.as_i64())
        .ok_or("roles sync response missing updated count")
}

/// The notification shown after a successful role sync.
pub(super) fn roles_sync_success_message(updated: i64) -> String {
    format!("Discord roles resynced ({updated} user(s) updated)")
}

/// The personnel screen, behind the administrator gate.
#[component]
pub fn PersonnelRosterPage() -> impl IntoView {
    view! {
        <AdminGate>
            <PersonnelInner />
        </AdminGate>
    }
}

/// The screen an administrator sees: the fetch, the controls and the two panes.
#[component]
fn PersonnelInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let url_query = leptos_router::hooks::use_query_map();
    let query = Memo::new(move |_| {
        url_query.with(|params| {
            RosterQuery::from_url_values(
                params.get_str("page"),
                params.get_str("per_page"),
                params.get_str("q"),
            )
        })
    });
    let pathname = leptos_router::hooks::use_location().pathname;
    let navigate = leptos_router::hooks::use_navigate();
    let show_query = Callback::new(move |next: RosterQuery| {
        let href = format!("{}{}", pathname.get_untracked(), next.to_url_query());
        navigate(
            &href,
            NavigateOptions {
                replace: true,
                scroll: false,
                ..Default::default()
            },
        );
    });
    let api_path = Memo::new(move |_| query.get().api_path());
    let selected_id = RwSignal::new(None::<String>);
    let roster = LocalResource::new(move || {
        let path = api_path.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                crate::v2::core::api::client::api_get::<PersonnelPage>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, path);
                None::<PersonnelPage>
            }
        }
    });
    let position = Memo::new(move |_| {
        roster
            .get()
            .flatten()
            .map(|served| PagerPosition::served(&served))
    });
    Effect::new(move |_| {
        let Some(served) = position.get() else {
            return;
        };
        if let Some(first_page) = served.fallback_for(&query.get_untracked()) {
            show_query.run(first_page);
        }
    });
    let refetch = Callback::new(move |()| roster.refetch());
    let sync_busy = RwSignal::new(false);
    let sort_mode = RwSignal::new(SortMode::NameAsc);
    let filter_mode = RwSignal::new(FilterMode::All);
    let on_sync_roles = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if sync_busy.get_untracked() {
                return;
            }
            sync_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post::<serde_json::Value>(
                    store,
                    ADMIN_ROLES_SYNC_PATH,
                    serde_json::json!({}),
                )
                .await
                {
                    Ok(body) => match roles_sync_updated_count(&body) {
                        Ok(n) => {
                            toasts.success(roles_sync_success_message(n));
                            refetch.run(());
                        }
                        Err(_) => {
                            toasts.error("Role sync returned an unexpected response");
                        }
                    },
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Role sync failed",
                    )),
                }
                sync_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, sync_busy, refetch);
        }
    };
    let on_cycle_sort = move |_| sort_mode.update(|m| *m = m.next());
    let on_cycle_filter = move |_| filter_mode.update(|m| *m = m.next());
    view! {
        <div class="flex h-full w-full flex-1 overflow-hidden bg-surface-glass backdrop-blur-xl">
            // ── Left: data table (70%) ──
            <div class="flex min-w-0 flex-[7] flex-col border-r border-white/10">
                <div class="border-b border-white/5 p-6">
                    <div class="flex flex-wrap items-center justify-between gap-4">
                        <h1 class="text-headline-lg text-on-surface">"Personnel Roster"</h1>
                        <div class="flex items-center gap-2">
                            <button
                                type="button"
                                on:click=on_sync_roles
                                prop:disabled=move || sync_busy.get()
                                class="flex items-center gap-1.5 rounded-full border border-primary/40 bg-primary/10 px-4 py-2 text-label-sm text-primary transition hover:bg-primary/20 disabled:opacity-50"
                            >
                                <MaterialIcon name="sync" class="text-[18px]" />
                                {move || {
                                    if sync_busy.get() {
                                        "Syncing…"
                                    } else {
                                        "Sync Roles"
                                    }
                                }}
                            </button>
                            <button
                                type="button"
                                on:click=on_cycle_sort
                                title="Sorts the members on this page"
                                class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="swap_vert" class="text-[18px]" />
                                {move || sort_mode.get().label()}
                            </button>
                            <button
                                type="button"
                                on:click=on_cycle_filter
                                title="Filters the members on this page"
                                class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="filter_list" class="text-[18px]" />
                                {move || filter_mode.get().label()}
                            </button>
                        </div>
                    </div>
                    <div class="relative mt-4">
                        <MaterialIcon
                            name="search"
                            class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-[18px] text-on-surface-variant"
                        />
                        <input
                            type="search"
                            placeholder="Search Discord ID or Arma Name…"
                            // The empty attribute is kept alongside the property binding: the
                            // rendered markup is pinned with it present.
                            value=""
                            prop:value=move || query.get().q
                            on:input=move |ev| {
                                let search = event_target_value(&ev);
                                show_query.run(query.get_untracked().with_search(search))
                            }
                            class="w-full max-w-md rounded-full border border-white/10 bg-black/20 py-2.5 pr-3 pl-9 text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:border-primary/50"
                        />
                    </div>
                    <p
                        data-testid="personnel-page-local-caption"
                        class="mt-2 text-label-sm text-on-surface-variant"
                    >
                        "Search covers the whole roster; sort and filter act on this page only."
                    </p>
                </div>
                <div class="custom-scrollbar min-h-0 flex-1 overflow-y-auto">
                    <Suspense fallback=move || {
                        view! { <p class="text-on-surface-variant">"Loading…"</p> }
                    }>
                        {move || {
                            let sort = sort_mode.get();
                            let filter = filter_mode.get();
                            roster
                                .get()
                                .map(|opt| match opt {
                                    Some(served) => {
                                        let users = apply_roster_sort(
                                            apply_roster_filter(served.items, filter),
                                            sort,
                                        );
                                        roster_table(users, selected_id).into_any()
                                    }
                                    None => {
                                        view! { <p class="text-error">"Failed to load data."</p> }
                                            .into_any()
                                    }
                                })
                        }}
                    </Suspense>
                </div>
                {roster_pager(position, query, show_query)}
            </div>

            // ── Right: fixed dossier (30%) ──
            <aside class="flex min-w-0 flex-[3] flex-col bg-surface-container-lowest/40">
                <Suspense fallback=move || ()>
                    {move || {
                        let sel = selected_id.get();
                        let user = roster
                            .get()
                            .flatten()
                            .and_then(|served| {
                                served
                                    .items
                                    .iter()
                                    .find(|u| Some(&u.discord_id) == sel.as_ref())
                                    .cloned()
                            });
                        match user {
                            Some(u) => dossier(u, refetch).into_any(),
                            None => {
                                view! {
                                    <div class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center text-on-surface-variant">
                                        <MaterialIcon name="badge" class="text-4xl opacity-50" />
                                        <p class="text-label-md">
                                            "Select personnel to view dossier"
                                        </p>
                                    </div>
                                }
                                    .into_any()
                            }
                        }
                    }}
                </Suspense>
            </aside>
        </div>
    }
}
