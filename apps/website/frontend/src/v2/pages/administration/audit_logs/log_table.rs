//! The trail: one line per audit entry, the page-by-page load, and the entry inspector.
//!
//! **Role:** the accumulated list of entries with its filter and load control, and the expanded
//! view of whichever entry is selected.
//! **Position:** the two panes of the audit route, below the filter box.
//! **Signals & state:** owns `lines` (the accumulated trail), `next_cursor` (where the next page
//! starts, or nothing at the end), `loading_more` and `load_more_error` around that request,
//! `selected` (which entry is expanded) and `query` (the filter text).
//! **Invariants:** loading more **appends**: replacing the trail with the new page would silently
//! truncate everything already read. The filter runs over what is loaded, so an empty result means
//! the filter matched nothing, not that the trail is empty — the two say different things on
//! screen. An entry carries every field the inspector shows, so opening one never fetches again and
//! cannot show one entry's identity under another's details. A filter can hide the selected entry
//! but never remove it.
#![allow(dead_code)]

use super::filter_bar::{filter_bar, haystack};
#[cfg(target_arch = "wasm32")]
use super::page::{audit_logs_path, merge_audit_page};
use super::page::{parse_next_cursor, vid, vstr};
use crate::v2::core::api::dto::CursorList;
use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::split_pane::{search_matches, SplitPane, SplitPaneEmpty};
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::datefmt::log_stamp;
use leptos::prelude::*;
use serde_json::Value;

/// The level token shown against an entry.
///
/// An unrecognised severity is shown upper-cased rather than silently relabelled, so a level this
/// screen has not heard of is visible instead of disguised as routine.
pub(super) fn level_label(severity: &str) -> String {
    match severity {
        "info" => "INFO".into(),
        "warn" => "WARN".into(),
        "crit" => "CRIT".into(),
        "" => "----".into(),
        other => other.to_uppercase(),
    }
}

/// The colour the level token is drawn in.
pub(super) fn level_class(severity: &str) -> &'static str {
    match severity {
        "warn" => "shrink-0 text-tactical-yellow",
        "crit" => "shrink-0 font-bold text-error-alert",
        _ => "shrink-0 text-primary",
    }
}

/// The badge variant an entry's severity is shown with in the inspector.
pub(super) fn severity_variant(severity: &str) -> &'static str {
    match severity {
        "warn" => "warning",
        "crit" => "error",
        "info" => "primary",
        _ => "neutral",
    }
}

/// The trail beside the entry inspector.
pub(super) fn board(store: AuthStore, page: CursorList<Value>) -> impl IntoView {
    // The trail grows page by page; the filter runs on whatever is already loaded rather than
    // re-querying the server.
    let lines = RwSignal::new(page.data);
    let next_cursor = RwSignal::new(parse_next_cursor(&page.next_cursor));
    let loading_more = RwSignal::new(false);
    let load_more_error = RwSignal::new(false);
    let selected = RwSignal::new(None::<i64>);
    let query = RwSignal::new(String::new());

    let on_load_more = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(before) = next_cursor.get_untracked() else {
                return;
            };
            if loading_more.get_untracked() {
                return;
            }
            loading_more.set(true);
            load_more_error.set(false);
            let path = audit_logs_path(Some(before));
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_get::<CursorList<Value>>(store, &path).await
                {
                    Ok(page) => {
                        let mut rows = lines.get_untracked();
                        let cursor = merge_audit_page(&mut rows, page);
                        lines.set(rows);
                        next_cursor.set(cursor);
                    }
                    Err(_) => {
                        load_more_error.set(true);
                    }
                }
                loading_more.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, next_cursor, loading_more, load_more_error, lines);
        }
    };

    let master_header = filter_bar(query);
    let list = view! {
        {move || {
            let q = query.get();
            let rows_owned = lines.get();
            if rows_owned.is_empty() {
                return view! {
                    <p class="px-1 py-4 text-on-surface-variant">"No audit logs."</p>
                }
                    .into_any();
            }
            let rows: Vec<&Value> = rows_owned
                .iter()
                .filter(|l| search_matches(&q, &haystack(l)))
                .collect();
            if rows.is_empty() {
                // The page HAS entries; this query matched none of them. Saying "No audit
                // logs." here would read as an empty trail rather than an empty filter.
                return view! {
                    <p class="px-1 py-4 text-on-surface-variant">
                        "No entries match this filter."
                    </p>
                }
                    .into_any();
            }
            rows.into_iter()
                .map(|l| {
                    let id = vid(l);
                    let sev = vstr(l, "severity");
                    let stamp = log_stamp(&vstr(l, "created_at"));
                    let level = level_label(&sev);
                    let lvl_class = level_class(&sev);
                    let action = vstr(l, "action");
                    let message = vstr(l, "message");
                    view! {
                        <button
                            type="button"
                            on:click=move |_| selected.set(Some(id))
                            class=move || {
                                crate::v2::core::ui::cn(
                                    &[
                                        "flex w-full items-start gap-2 rounded px-2 py-1 text-left transition",
                                        if selected.get() == Some(id) {
                                            "bg-primary/15 text-on-surface shadow-[inset_2px_0_0_0_#adc6ff]"
                                        } else {
                                            "text-on-surface-variant hover:bg-white/[0.04] hover:text-on-surface"
                                        },
                                    ],
                                )
                            }
                        >
                            <span class="shrink-0 text-outline">{stamp}</span>
                            <span class=lvl_class>"["{level}"]"</span>
                            <span class="shrink-0 text-tertiary">{action}</span>
                            <span class="min-w-0 flex-1 truncate">{message}</span>
                        </button>
                    }
                })
                .collect_view()
                .into_any()
        }}
    };

    let load_more = view! {
        {move || {
            if next_cursor.get().is_none() {
                return ().into_any();
            }
            view! {
                <div class="mt-3 flex flex-col items-start gap-2 px-1">
                    <button
                        type="button"
                        on:click=on_load_more
                        prop:disabled=move || loading_more.get()
                        class="rounded-lg border border-primary/40 bg-primary/10 px-3 py-1.5 font-mono text-xs tracking-widest text-primary uppercase transition hover:bg-primary/20 disabled:opacity-50"
                    >
                        {move || {
                            if loading_more.get() {
                                "Loading…"
                            } else {
                                "Load more"
                            }
                        }}
                    </button>
                    {move || {
                        load_more_error
                            .get()
                            .then(|| {
                                view! {
                                    <p class="font-mono text-xs text-error-alert">
                                        "Could not load the next page."
                                    </p>
                                }
                            })
                    }}
                </div>
            }
                .into_any()
        }}
    };

    let master = view! {
        <div class="font-mono text-code-md">
            {list} {load_more}
            <span class="ml-2 inline-block h-3 w-2 animate-pulse bg-primary align-middle"></span>
        </div>
    }
    .into_any();

    let detail = view! {
        {move || {
            let Some(id) = selected.get() else {
                return view! {
                    <SplitPaneEmpty
                        icon=view! { <MaterialIcon name="terminal" class="text-4xl" /> }.into_any()
                        message="Select a log entry to inspect."
                    />
                }
                    .into_any();
            };
            let rows = lines.get();
            match rows.iter().find(|l| vid(l) == id) {
                Some(l) => entry(l).into_any(),
                // A filter can hide the selected row but never delete it; this only fires
                // if the page is ever replaced under a live selection.
                None => {
                    view! {
                        <SplitPaneEmpty
                            icon=view! { <MaterialIcon name="terminal" class="text-4xl" /> }
                                .into_any()
                            message="That entry is no longer in this page of the trail."
                        />
                    }
                        .into_any()
                }
            }
        }}
    }
    .into_any();

    view! {
        <SplitPane
            master_width="60%"
            master_header=master_header
            master=master
            detail=detail
        />
    }
}

/// One expanded audit entry.
///
/// Everything but the identifier, the action, the stamp, the message and the severity is optional
/// on the wire — a server event has no actor, a sign-in has no target — so each optional row is
/// rendered only when it is present rather than shown as an empty field.
pub(super) fn entry(l: &Value) -> impl IntoView + use<> {
    let sev = vstr(l, "severity");
    let action = vstr(l, "action");
    let message = vstr(l, "message");
    let stamp = log_stamp(&vstr(l, "created_at"));
    let actor_name = vstr(l, "actor_name");
    let actor_id = vstr(l, "actor_id");
    let target_type = vstr(l, "target_type");
    let target_id = vstr(l, "target_id");
    let id = vid(l);
    // `metadata` is free-form jsonb. Pretty-printed as JSON rather than guessed at per action —
    // the vocabulary differs for every action and the operator reading an audit trail wants the
    // raw record, not a paraphrase.
    let metadata = l
        .get("metadata")
        .filter(|m| !m.is_null())
        .and_then(|m| serde_json::to_string_pretty(m).ok());
    view! {
        <div class="flex flex-col gap-6 px-8 py-8">
            <header class="flex flex-col gap-3 border-b border-outline-variant/30 pb-5">
                <div class="flex flex-wrap items-center gap-2">
                    <span class=badge_class(severity_variant(&sev))>{level_label(&sev)}</span>
                    <span class="font-mono text-code-md text-tertiary">{action}</span>
                </div>
                <p class="text-body-md leading-relaxed text-on-surface">{message}</p>
                <span class="font-mono text-xs text-outline">{stamp}</span>
            </header>
            <dl class="flex flex-col gap-3 font-mono text-code-md">
                <EntryField label="Entry" value=id.to_string() />
                {(!actor_name.is_empty() || !actor_id.is_empty())
                    .then(|| {
                        let who = if actor_name.is_empty() {
                            actor_id.clone()
                        } else {
                            format!("{actor_name} ({actor_id})")
                        };
                        view! { <EntryField label="Actor" value=who /> }
                    })}
                {(!target_type.is_empty())
                    .then(|| view! { <EntryField label="Target type" value=target_type.clone() /> })}
                {(!target_id.is_empty())
                    .then(|| view! { <EntryField label="Target id" value=target_id.clone() /> })}
            </dl>
            {metadata
                .map(|m| {
                    view! {
                        <section class="flex flex-col gap-2">
                            <h3 class="font-mono text-xs tracking-widest text-on-surface-variant uppercase">
                                "Metadata"
                            </h3>
                            <pre class="custom-scrollbar overflow-x-auto rounded-lg border border-white/10 bg-black/30 p-4 font-mono text-code-md text-on-surface-variant">
                                {m}
                            </pre>
                        </section>
                    }
                })}
        </div>
    }
}

/// One labelled field inside the entry inspector.
#[component]
fn EntryField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="flex items-baseline gap-3">
            <dt class="w-28 shrink-0 text-xs tracking-widest text-on-surface-variant uppercase">
                {label}
            </dt>
            <dd class="min-w-0 break-all text-on-surface">{value}</dd>
        </div>
    }
}
