//! The content route: the posts an administrator has written, and the one being edited.
//!
//! **Role:** fetches the post list, seeds the working set from it once, and arranges the master
//! list beside the editor form.
//! **Position:** the `/admin/content` route, rendered inside the navigation frame.
//! **Signals & state:** `docs` is the working set — seeded from the fetch, then mutated by New,
//! Publish and Delete; `selected_id` is the post open in the editor; `list_seeded` marks the
//! one-shot hydrate as done; `list_error` carries a failed fetch to the master pane;
//! `publish_busy` and `delete_busy` gate the editor's controls.
//! **Invariants:** the fetch keeps its result rather than collapsing a failure into an empty page —
//! a failure must not look like a catalogue with nothing in it. The hydrate runs **once and only on
//! success**, so a later read cannot overwrite a local draft, and a failure leaves the flag unset so
//! the retry can re-enter it. A failed fetch shows an actionable message and a retry in the master
//! pane; a genuinely empty catalogue shows the empty copy instead. New posts are held locally until
//! their first publish, which is what mints a server identifier for them.
#![allow(dead_code)]

use super::article_table::article_row;
#[cfg(target_arch = "wasm32")]
use super::doc::announcement_list_path;
use super::doc::{doc_from_announcement, Doc};
use super::editor_form::editor;
#[cfg(target_arch = "wasm32")]
use super::editor_form::today_iso;
use crate::v2::core::api::dto::Paginated;
use crate::v2::core::ui::split_pane::{SplitPane, SplitPaneEmpty};
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::Value;

/// The content screen, behind the administrator gate.
#[component]
pub fn ContentManagerPage() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &store;
    // The working set: seeded once from the fetch, then mutated by New, Publish and Delete.
    let docs = RwSignal::new(Vec::<Doc>::new());
    let selected_id = RwSignal::new(None::<String>);
    let list_seeded = RwSignal::new(false);
    let list_error = RwSignal::new(None::<String>);
    let publish_busy = RwSignal::new(false);
    let delete_busy = RwSignal::new(false);

    // The result is kept rather than discarded: collapsing a failure into an absence would make
    // the hydrate treat it as a successful empty page.
    let list_res = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<Paginated<Value>>(
                store,
                announcement_list_path(),
            )
            .await
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            Err::<Paginated<Value>, crate::v2::core::api::client::ApiErr>((
                0,
                Some("CMS list unavailable off wasm".into()),
            ))
        }
    });

    // Hydrates once, so a later read cannot overwrite a local draft or a published edit. Only a
    // success sets the flag, so a failure leaves the retry able to re-enter this.
    Effect::new(move |_| {
        if list_seeded.get() {
            return;
        }
        let Some(result) = list_res.get() else {
            return;
        };
        match result {
            Ok(page) => {
                list_error.set(None);
                list_seeded.set(true);
                let mapped: Vec<Doc> = page.data.iter().filter_map(doc_from_announcement).collect();
                if selected_id.get_untracked().is_none() {
                    selected_id.set(mapped.first().map(|d| d.id.clone()));
                }
                docs.set(mapped);
            }
            Err(e) => {
                list_error.set(Some(crate::v2::core::api::client::api_error_message(
                    &e,
                    "Failed to load announcements",
                )));
            }
        }
    });

    let retry_list = move |_| {
        list_error.set(None);
        list_res.refetch();
    };

    let new_post = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let id = format!("new-{}", js_sys::Date::now() as u64);
            let doc = Doc {
                id: id.clone(),
                title: "Untitled Post".into(),
                category: "announcement".into(),
                published: false,
                date: today_iso(),
                body: String::new(),
                thumbnail_url: String::new(),
            };
            docs.update(|d| d.insert(0, doc));
            selected_id.set(Some(id));
        }
    };

    view! {
        <crate::v2::core::ui::AdminGate>
            <div class="relative h-full w-full overflow-hidden">
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                    <Suspense fallback=move || {
                        view! {
                            <p class="p-6 text-on-surface-variant">"Loading…"</p>
                        }
                    }>
                        {move || {
                            // Touch the resource so Suspense waits for the first fetch.
                            let _ = list_res.get();
                            view! {
                                <SplitPane
                                    transparent=true
                                    master_width="20rem"
                                    master_header=view! {
                                        <>
                                            <h1 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                                                "Comms Broadcaster"
                                            </h1>
                                            <button
                                                type="button"
                                                on:click=new_post
                                                class="flex shrink-0 items-center gap-1.5 rounded-full border border-white/10 px-3 py-1.5 text-label-sm text-on-surface transition hover:bg-white/5"
                                            >
                                                <MaterialIcon name="add" class="text-[18px]" />
                                                "New"
                                            </button>
                                        </>
                                    }
                                        .into_any()
                                    master=view! {
                                        {move || {
                                            if let Some(err) = list_error.get() {
                                                return view! {
                                                    <div class="flex flex-col gap-3 px-1 py-4">
                                                        <p
                                                            class="text-label-md text-error"
                                                            data-testid="content-list-error"
                                                        >
                                                            {err}
                                                        </p>
                                                        <button
                                                            type="button"
                                                            data-testid="content-list-retry"
                                                            on:click=retry_list
                                                            class="self-start rounded-full border border-white/10 px-3 py-1.5 text-label-sm text-on-surface transition hover:bg-white/5"
                                                        >
                                                            "Retry"
                                                        </button>
                                                    </div>
                                                }
                                                    .into_any();
                                            }
                                            let sel = selected_id.get();
                                            let rows = docs.get();
                                            if rows.is_empty() {
                                                return view! {
                                                    <p class="px-1 py-4 text-label-md text-on-surface-variant">
                                                        "No announcements yet."
                                                    </p>
                                                }
                                                    .into_any();
                                            }
                                            rows
                                                .into_iter()
                                                .map(|d| {
                                                    let active = sel.as_deref()
                                                        == Some(d.id.as_str());
                                                    article_row(d, active, selected_id)
                                                })
                                                .collect_view()
                                                .into_any()
                                                .into_any()
                                        }}
                                    }
                                        .into_any()
                                    detail=view! {
                                        {move || {
                                            let sel = selected_id.get();
                                            let doc = docs
                                                .get()
                                                .into_iter()
                                                .find(|d| Some(&d.id) == sel.as_ref());
                                            match doc {
                                                Some(d) => {
                                                    editor(
                                                        d,
                                                        docs,
                                                        selected_id,
                                                        publish_busy,
                                                        delete_busy,
                                                        store,
                                                    )
                                                        .into_any()
                                                }
                                                None => {
                                                    view! {
                                                        <SplitPaneEmpty
                                                            icon=view! {
                                                                <MaterialIcon name="edit_note" class="text-4xl" />
                                                            }
                                                                .into_any()
                                                            message="Select a post or create a new one."
                                                        />
                                                    }
                                                        .into_any()
                                                }
                                            }
                                        }}
                                    }
                                        .into_any()
                                />
                            }
                                .into_any()
                        }}
                    </Suspense>
                </div>
            </div>
        </crate::v2::core::ui::AdminGate>
    }
}
