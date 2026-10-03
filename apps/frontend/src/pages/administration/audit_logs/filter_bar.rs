//! The filter box above the trail, and what it matches on.
//!
//! **Role:** the search field over the master pane, and the text one entry is matched against.
//! **Position:** the master header of the audit route, beside the live status badge.
//! **Signals & state:** writes `query`, which the trail reads to filter what it shows.
//! **Invariants:** filtering happens **on what is already loaded**, not on the server. Re-keying
//! the fetch on every keystroke would send a request per character and risk showing a stale answer,
//! and the endpoint serves a whole page at a time in any case. The haystack is the stamp, the
//! level, the action, the actor, the message and the target type — what an operator would read
//! down the column.

#[cfg(target_arch = "wasm32")]
use super::log_table::level_label;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::administration::AuditLogEntry;
#[cfg(target_arch = "wasm32")]
use crate::foundation::utils::datefmt::log_stamp;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Everything the filter box matches one entry against, as one string.
#[cfg(target_arch = "wasm32")]
pub(super) fn haystack(entry: &AuditLogEntry) -> String {
    format!(
        "{} {} {} {} {} {}",
        log_stamp(&entry.created_at),
        level_label(entry.severity),
        entry.action,
        entry.actor_name,
        entry.message,
        entry.target_type,
    )
}

/// The search field shown above the trail.
#[cfg(target_arch = "wasm32")]
pub(super) fn filter_bar(query: RwSignal<String>) -> AnyView {
    view! {
        <input
            type="search"
            placeholder="Filter by admin, action, or keyword..."
            value=""
            on:input=move |ev| query.set(event_target_value(&ev))
            class="w-full rounded-lg border border-outline-variant/40 bg-surface-container px-3 py-1.5 font-mono text-code-md outline-none focus:border-primary/60"
        />
    }
    .into_any()
}
