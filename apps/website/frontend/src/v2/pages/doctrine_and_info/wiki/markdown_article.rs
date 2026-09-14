//! The detail half of the wiki: one manual, read or edited.
//!
//! **Role:** renders the open manual — its category, last-updated chip and title, the rendered
//! body, and for an administrator the read/edit switch, the raw-source editor and the save
//! button that `PUT`s the page back.
//! **Position:** the detail pane of the wiki's split view.
//! **Signals & state:** takes the page's `mode`, its per-slug `drafts` map, and the `save_busy`
//! and `save_err` signals; reads the `AuthStore` from context through its caller and refetches
//! the manual list resource after a successful save.
//! **Invariants:** a draft outlives a switch back to reading, so the rendered body shows the
//! unsaved edit; the draft is dropped only once the save succeeds. The save path exists on
//! `wasm32` only — natively the button clears its busy flag and does nothing.

use super::helpers::{vi64, vstr};
use super::markdown::render_markdown;
use super::page::WikiMode;
use crate::v2::core::api::dto::DataEnvelope;
use leptos::prelude::*;
use serde_json::Value;

/// The neutral chip used for the "last updated" stamp.
const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";

/// The `YYYY-MM-DD` prefix of an ISO timestamp, or an em dash when there is none.
///
/// Deliberately string slicing rather than a date library: this runs in the native unit tests as
/// well as in the browser.
pub(super) fn updated_day(iso: &str) -> String {
    let day = iso.get(..10).unwrap_or("");
    if day.len() == 10 && day.as_bytes().get(4) == Some(&b'-') {
        day.to_string()
    } else {
        "—".into()
    }
}

/// The open manual.
///
/// `page` is the row to render, `mode` the read/edit switch, `drafts` the per-slug unsaved
/// bodies, `save_busy` and `save_err` the state of the in-flight `PUT`, `is_admin` whether the
/// editing affordances are shown at all, `store` the session used for that request, and
/// `pages_res` the list resource refetched once a save lands.
pub(super) fn article(
    page: Value,
    mode: RwSignal<WikiMode>,
    drafts: RwSignal<std::collections::HashMap<String, String>>,
    save_busy: RwSignal<bool>,
    save_err: RwSignal<Option<String>>,
    is_admin: bool,
    store: crate::v2::core::auth::AuthStore,
    pages_res: LocalResource<Option<DataEnvelope<Value>>>,
) -> impl IntoView {
    let slug = vstr(&page, "slug");
    let title = vstr(&page, "title");
    let category = vstr(&page, "category");
    let icon = vstr(&page, "icon");
    let nav_order = vi64(&page, "nav_order");
    let body_md = vstr(&page, "body_md");
    let updated = updated_day(&vstr(&page, "updated_at"));
    let slug_for_draft = slug.clone();
    let body_for_read = body_md.clone();

    view! {
        <section class="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
            <header class="flex shrink-0 items-start justify-between gap-4 border-b border-white/10 px-8 pt-8 pb-5 md:px-12">
                <div class="min-w-0">
                    <div class="mb-3 flex items-center gap-2">
                        <span class=BADGE_NEUTRAL>
                            <span class="material-symbols-outlined text-[14px]">"schedule"</span>
                            "Last updated "
                            {updated}
                        </span>
                        <span class="font-mono text-xs tracking-widest text-outline uppercase">
                            {category.clone()}
                        </span>
                    </div>
                    <h1 class="text-4xl font-bold tracking-tight text-white">{title.clone()}</h1>
                </div>
                {if is_admin {
                    view! {
                        <div class="flex shrink-0 flex-col items-end gap-2">
                            {read_edit_toggle(mode)}
                            {move || {
                                if mode.get() == WikiMode::Edit {
                                    let slug_put = slug.clone();
                                    let title_put = title.clone();
                                    let category_put = category.clone();
                                    let icon_put = icon.clone();
                                    let body_fallback = body_md.clone();
                                    view! {
                                        <button
                                            type="button"
                                            disabled=move || save_busy.get()
                                            class="rounded-full border border-primary/40 bg-primary/15 px-4 py-1.5 font-mono text-xs tracking-widest text-primary uppercase hover:bg-primary/25 disabled:opacity-50"
                                            on:click=move |_| {
                                                if save_busy.get_untracked() {
                                                    return;
                                                }
                                                save_busy.set(true);
                                                save_err.set(None);
                                                let draft = drafts
                                                    .with_untracked(|d| d.get(&slug_put).cloned())
                                                    .unwrap_or_else(|| body_fallback.clone());
                                                let body = serde_json::json!({
                                                    "category": category_put.clone(),
                                                    "title": title_put.clone(),
                                                    "icon": icon_put.clone(),
                                                    "body_md": draft,
                                                    "nav_order": nav_order,
                                                });
                                                let path = format!("/wiki/{slug_put}");
                                                let slug_clear = slug_put.clone();
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    leptos::task::spawn_local(async move {
                                                        match crate::v2::core::api::client::api_put::<Value>(
                                                            store, &path, body,
                                                        )
                                                        .await
                                                        {
                                                            Ok(_) => {
                                                                drafts.update(|d| {
                                                                    d.remove(&slug_clear);
                                                                });
                                                                mode.set(WikiMode::Read);
                                                                pages_res.refetch();
                                                            }
                                                            Err(e) => {
                                                                save_err.set(Some(
                                                                    crate::v2::core::api::client::api_error_message(
                                                                        &e,
                                                                        "Failed to save wiki page",
                                                                    ),
                                                                ));
                                                            }
                                                        }
                                                        save_busy.set(false);
                                                    });
                                                }
                                                #[cfg(not(target_arch = "wasm32"))]
                                                {
                                                    let _ = (store, path, body, pages_res, slug_clear);
                                                    save_busy.set(false);
                                                }
                                            }
                                        >
                                            {move || {
                                                if save_busy.get() { "Saving…" } else { "Save" }
                                            }}
                                        </button>
                                        {move || {
                                            save_err
                                                .get()
                                                .map(|m| {
                                                    view! {
                                                        <p class="max-w-xs text-right font-mono text-[11px] text-error-alert">
                                                            {m}
                                                        </p>
                                                    }
                                                })
                                        }}
                                    }
                                        .into_any()
                                } else {
                                    ().into_any()
                                }
                            }}
                        </div>
                    }
                        .into_any()
                } else {
                    ().into_any()
                }}
            </header>
            {move || {
                if mode.get() == WikiMode::Edit {
                    let initial = drafts
                        .with_untracked(|e| e.get(&slug_for_draft).cloned())
                        .unwrap_or_else(|| body_for_read.clone());
                    let key = slug_for_draft.clone();
                    view! {
                        <textarea
                            prop:value=initial
                            spellcheck="false"
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                drafts.update(|e| {
                                    e.insert(key.clone(), v);
                                });
                            }
                            class="h-full w-full flex-1 resize-none border-none bg-transparent p-8 font-mono text-sm leading-relaxed text-on-surface-variant outline-none focus:ring-0 md:p-12"
                        ></textarea>
                    }
                        .into_any()
                } else {
                    let source = drafts
                        .with(|e| e.get(&slug_for_draft).cloned())
                        .unwrap_or_else(|| body_for_read.clone());
                    view! {
                        <article class="custom-scrollbar flex-1 overflow-y-auto p-8 md:p-12">
                            <div class="max-w-3xl">{render_markdown(&source)}</div>
                        </article>
                    }
                        .into_any()
                }
            }}
        </section>
    }
}

/// The read/edit switch shown to administrators.
fn read_edit_toggle(mode: RwSignal<WikiMode>) -> impl IntoView {
    let btn = |m: WikiMode, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || {
                    if mode.get() == m {
                        "rounded-full px-3 py-1 font-medium transition-all bg-surface-glass text-on-surface shadow-md"
                    } else {
                        "rounded-full px-3 py-1 font-medium transition-all text-on-surface-variant hover:text-on-surface"
                    }
                }
                on:click=move |_| mode.set(m)
            >
                {label}
            </button>
        }
    };
    view! {
        <div class="inline-flex shrink-0 gap-1 rounded-full border border-white/5 bg-black/20 p-1 font-mono text-xs">
            {btn(WikiMode::Read, "[ READ ]")}
            {btn(WikiMode::Edit, "[ EDIT ]")}
        </div>
    }
}
