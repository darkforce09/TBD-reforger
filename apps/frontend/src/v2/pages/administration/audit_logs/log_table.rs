//! The trail: one line per audit entry, the page-by-page load, and the entry inspector.
//!
//! **Role:** the merged board with its filter, live status and load control, and the expanded
//! view of whichever entry is selected.
//! **Position:** the two panes of the audit route; the route hands in the board and the stream
//! and history states it drives.
//! **Signals & state:** reads the route's `board`, stream state, history state and rejected-event
//! note; owns `loading_more` and `load_more_error` around a further page, `selected` (which entry
//! is expanded) and `query` (the filter text).
//! **Invariants:** loading more **merges** a page below the smallest history id into the board:
//! replacing the board with the new page would silently truncate everything already read, and a
//! page requested before a reload is dropped rather than mixed into the new history. The filter
//! runs over what is loaded, so an empty result means the filter matched nothing, not that the
//! trail is empty — the two say different things on screen. An entry carries every field the
//! inspector shows, so opening one never fetches again and cannot show one entry's identity under
//! another's details. A filter can hide the selected entry but never remove it.
#![allow(dead_code)]

use super::filter_bar::{filter_bar, haystack};
use super::live_merge::AuditBoard;
use super::live_status::{live_status, HistoryLoad};
#[cfg(target_arch = "wasm32")]
use super::page::audit_logs_path;
use crate::v2::core::api::audit_stream::AuditStreamState;
use crate::v2::core::api::dto::administration::{AuditLogEntry, AuditSeverity};
use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::split_pane::{search_matches, SplitPane, SplitPaneEmpty};
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::datefmt::log_stamp;
use leptos::prelude::*;

/// The level token shown against an entry.
pub(super) fn level_label(severity: AuditSeverity) -> &'static str {
    match severity {
        AuditSeverity::Info => "INFO",
        AuditSeverity::Warn => "WARN",
        AuditSeverity::Crit => "CRIT",
    }
}

/// The colour the level token is drawn in.
pub(super) fn level_class(severity: AuditSeverity) -> &'static str {
    match severity {
        AuditSeverity::Warn => "shrink-0 text-tactical-yellow",
        AuditSeverity::Crit => "shrink-0 font-bold text-error-alert",
        AuditSeverity::Info => "shrink-0 text-primary",
    }
}

/// The badge variant an entry's severity is shown with in the inspector.
pub(super) fn severity_variant(severity: AuditSeverity) -> &'static str {
    match severity {
        AuditSeverity::Warn => "warning",
        AuditSeverity::Crit => "error",
        AuditSeverity::Info => "primary",
    }
}

/// What the trail says when the board holds no line, by where the history stands.
pub(super) fn empty_board_message(history: HistoryLoad) -> &'static str {
    match history {
        HistoryLoad::Waiting | HistoryLoad::Loading | HistoryLoad::Reloading => "Loading…",
        HistoryLoad::Failed => "Failed to load data.",
        HistoryLoad::Loaded => "No audit logs.",
    }
}

/// The trail beside the entry inspector, fed by the route's board.
pub(super) fn board_view(
    store: AuthStore,
    board: RwSignal<AuditBoard>,
    stream: RwSignal<AuditStreamState>,
    history: RwSignal<HistoryLoad>,
    rejected: RwSignal<Option<String>>,
) -> impl IntoView {
    let loading_more = RwSignal::new(false);
    let load_more_error = RwSignal::new(false);
    let selected = RwSignal::new(None::<i64>);
    let query = RwSignal::new(String::new());

    let on_load_more = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let next = board.with_untracked(|b| b.continuation().map(|before| (before, b.epoch())));
            let Some((before, epoch)) = next else {
                return;
            };
            if loading_more.get_untracked() {
                return;
            }
            loading_more.set(true);
            load_more_error.set(false);
            let path = audit_logs_path(Some(before));
            leptos::task::spawn_local(async move {
                use crate::v2::core::api::dto::CursorList;
                let answer = crate::v2::core::api::client::api_get::<CursorList<AuditLogEntry>>(
                    store, &path,
                )
                .await;
                match answer {
                    Ok(page) => {
                        let _ = board.try_update(|b| b.merge_history(epoch, page));
                    }
                    Err(_) => {
                        let _ = load_more_error.try_set(true);
                    }
                }
                let _ = loading_more.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, board, loading_more, load_more_error);
        }
    };

    let master_header = view! {
        <div class="flex items-start gap-3">
            <div class="min-w-0 flex-1">{filter_bar(query)}</div>
            {live_status(stream, history, board, rejected)}
        </div>
    }
    .into_any();
    let list = view! {
        {move || {
            let q = query.get();
            let rows_owned = board.with(AuditBoard::rows);
            if rows_owned.is_empty() {
                return view! {
                    <p class="px-1 py-4 text-on-surface-variant">
                        {empty_board_message(history.get())}
                    </p>
                }
                    .into_any();
            }
            let rows: Vec<&AuditLogEntry> = rows_owned
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
                    let id = l.id;
                    let stamp = log_stamp(&l.created_at);
                    let level = level_label(l.severity);
                    let lvl_class = level_class(l.severity);
                    let action = l.action.clone();
                    let message = l.message.clone();
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
            if history.get() == HistoryLoad::Failed && !board.with(AuditBoard::is_empty) {
                return view! {
                    <p class="mt-3 px-1 font-mono text-xs text-error-alert">
                        "Failed to load data."
                    </p>
                }
                    .into_any();
            }
            if board.with(|b| b.continuation().is_none()) {
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
                        icon={view! { <MaterialIcon name="terminal" class="text-4xl" /> }.into_any()}
                        message="Select a log entry to inspect."
                    />
                }
                    .into_any();
            };
            match board.with(|b| b.get(id).cloned()) {
                Some(l) => entry(&l).into_any(),
                // A filter can hide the selected row but never delete it; this only fires
                // when a reload empties the board under a live selection.
                None => {
                    view! {
                        <SplitPaneEmpty
                            icon={view! { <MaterialIcon name="terminal" class="text-4xl" /> }
                                .into_any()}
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
pub(super) fn entry(l: &AuditLogEntry) -> impl IntoView + use<> {
    let sev = l.severity;
    let action = l.action.clone();
    let message = l.message.clone();
    let stamp = log_stamp(&l.created_at);
    let actor_name = l.actor_name.clone();
    let actor_id = l.actor_id.clone().unwrap_or_default();
    let target_type = l.target_type.clone();
    let target_id = l.target_id.clone();
    let id = l.id;
    // `metadata` is free-form jsonb. Pretty-printed as JSON rather than guessed at per action —
    // the vocabulary differs for every action and the operator reading an audit trail wants the
    // raw record, not a paraphrase.
    let metadata = l
        .metadata
        .as_ref()
        .filter(|m| !m.is_null())
        .and_then(|m| serde_json::to_string_pretty(m).ok());
    view! {
        <div class="flex flex-col gap-6 px-8 py-8">
            <header class="flex flex-col gap-3 border-b border-outline-variant/30 pb-5">
                <div class="flex flex-wrap items-center gap-2">
                    <span class={badge_class(severity_variant(sev))}>{level_label(sev)}</span>
                    <span class="font-mono text-code-md text-tertiary">{action}</span>
                </div>
                <p class="text-body-md leading-relaxed text-on-surface">{message}</p>
                <span class="font-mono text-xs text-outline">{stamp}</span>
            </header>
            <dl class="flex flex-col gap-3 font-mono text-code-md">
                <EntryField label="Entry" value={id.to_string()} />
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
                    .then(|| view! { <EntryField label="Target type" value={target_type.clone()} /> })}
                {(!target_id.is_empty())
                    .then(|| view! { <EntryField label="Target id" value={target_id.clone()} /> })}
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
