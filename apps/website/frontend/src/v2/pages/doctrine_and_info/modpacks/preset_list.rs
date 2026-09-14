//! The master half of the modpacks page: the header, its create button, and the list of packs.
//!
//! **Role:** renders the page title, the administrator's create action, the search box, and one
//! row per modpack showing its version, addon count, download size and whether it is the pack
//! the servers currently run.
//! **Position:** the master pane of the modpacks split view, and the header above it.
//! **Signals & state:** reads and writes the page's `search` and `selected_id` signals, reads
//! the `is_admin` memo, owns nothing but the `create_busy` flag it is handed; the `AuthStore`
//! comes from context and the pack list resource is refetched after a create.
//! **Invariants:** a newly created pack becomes the selection, so the detail pane follows the
//! create. The search matches a pack's name or any of its addon names. The create request runs
//! on `wasm32` only; natively the button clears its busy flag and does nothing.

use super::pack_edit::{format_bytes, vstr};
use crate::v2::core::api::dto::{DataEnvelope, ModpackDto};
use crate::v2::core::ui::split_pane::{ListDetailItem, SidebarSearch};
use leptos::prelude::*;
use serde_json::json;

/// The chip marking the modpack the servers currently run.
const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";

/// The page header: the title, the create button for an administrator, and the search box.
///
/// `search` is the live search text, `is_admin` gates the create button, `create_busy` disables
/// it while a request is in flight, `packs_res` is refetched once one lands, `selected_id`
/// receives the new pack's id, and `toasts` reports the outcome.
pub(super) fn master_header(
    search: RwSignal<String>,
    is_admin: Memo<bool>,
    create_busy: RwSignal<bool>,
    packs_res: LocalResource<Option<DataEnvelope<ModpackDto>>>,
    selected_id: RwSignal<String>,
    toasts: crate::v2::core::ui::toast::Toasts,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    view! {
        <div class="w-full space-y-3">
            <div class="flex items-center justify-between gap-2">
                <h1 class="text-headline-sm tracking-wide text-on-surface uppercase">"Modpacks"</h1>
                {move || {
                    is_admin.get().then(|| {
                        view! {
                            <button
                                type="button"
                                disabled=move || create_busy.get()
                                class="rounded-full border border-white/10 px-3 py-1 font-mono text-[11px] tracking-wider text-on-surface-variant uppercase transition hover:bg-white/5 disabled:opacity-50"
                                on:click=move |_| {
                                    if create_busy.get_untracked() {
                                        return;
                                    }
                                    create_busy.set(true);
                                    let body = json!({
                                        "name": "New Modpack",
                                        "version": "0.1.0",
                                        "total_size_bytes": 0,
                                        "workshop_url": "",
                                        "is_current": false,
                                        "mods": [],
                                    });
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        leptos::task::spawn_local(async move {
                                            match crate::v2::core::api::client::api_post::<ModpackDto>(
                                                store, "/modpacks", body,
                                            )
                                            .await
                                            {
                                                Ok(created) => {
                                                    selected_id.set(created.modpack.id.clone());
                                                    toasts.success(format!(
                                                        "Created \"{}\"",
                                                        created.modpack.name
                                                    ));
                                                    packs_res.refetch();
                                                }
                                                Err(e) => {
                                                    toasts.error(crate::v2::core::api::client::api_error_message(
                                                        &e,
                                                        "Failed to create modpack",
                                                    ));
                                                }
                                            }
                                            create_busy.set(false);
                                        });
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        let _ = (store, body, packs_res, selected_id, toasts);
                                        create_busy.set(false);
                                    }
                                }
                            >
                                {move || if create_busy.get() { "…" } else { "+ New" }}
                            </button>
                        }
                    })
                }}
            </div>
            <SidebarSearch placeholder="Search packs & mods…" bind=search />
        </div>
    }
}

/// The list of modpacks.
///
/// `packs` is the full list, `selected_id` the row the detail pane is showing, and `query` the
/// live search text.
pub(super) fn pack_list(
    packs: &[ModpackDto],
    selected_id: RwSignal<String>,
    query: &str,
) -> impl IntoView {
    let query = query.to_string();
    packs
        .iter()
        .filter(|p| {
            let mods: String = p
                .mods
                .iter()
                .map(|m| vstr(m, "name"))
                .collect::<Vec<_>>()
                .join(" ");
            crate::v2::core::ui::split_pane::search_matches(
                &query,
                &format!("{} {mods}", p.modpack.name),
            )
        })
        .cloned()
        .map(|p| {
            let trailing = if p.modpack.is_current {
                view! { <span class=BADGE_SUCCESS>"Active"</span> }.into_any()
            } else {
                ().into_any()
            };
            let preview = view! {
                <span class="font-mono text-on-surface-variant">
                    "v"
                    {p.modpack.version.clone()}
                    " · "
                    {p.mods.len() as i64}
                    " mods · "
                    {format_bytes(p.modpack.total_size_bytes)}
                </span>
            }
            .into_any();
            let id = p.modpack.id.clone();
            let id_click = id.clone();
            let title = p.modpack.name.clone();
            view! {
                <ListDetailItem
                    active=id == selected_id.get()
                    title=view! { {title} }.into_any()
                    trailing=trailing
                    preview=preview
                    on_click=Callback::new(move |()| selected_id.set(id_click.clone()))
                />
            }
        })
        .collect_view()
}
