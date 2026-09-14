//! The filter box above the trail, and what it matches on.
//!
//! **Role:** the search field over the master pane, and the text one entry is matched against.
//! **Position:** the master header of the audit route.
//! **Signals & state:** writes `query`, which the trail reads to filter what it shows.
//! **Invariants:** filtering happens **on what is already loaded**, not on the server. Re-keying
//! the fetch on every keystroke would send a request per character and risk showing a stale answer,
//! and the endpoint serves a whole page at a time in any case. The haystack is the stamp, the
//! level, the action, the actor and the message — what an operator would read down the column.
#![allow(dead_code)]

use super::log_table::level_label;
use super::page::vstr;
use crate::v2::core::utils::datefmt::log_stamp;
use leptos::prelude::*;
use serde_json::Value;

/// Everything the filter box matches one entry against, as one string.
pub(super) fn haystack(l: &Value) -> String {
    let sev = vstr(l, "severity");
    format!(
        "{} {} {} {} {} {}",
        log_stamp(&vstr(l, "created_at")),
        level_label(&sev),
        vstr(l, "action"),
        vstr(l, "actor_name"),
        vstr(l, "message"),
        vstr(l, "target_type"),
    )
}

/// The search field shown above the trail.
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
