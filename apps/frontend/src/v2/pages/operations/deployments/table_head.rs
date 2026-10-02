//! The column heading both tables on this page are built from.
//!
//! **Role:** renders one `<th>` in the page's shared heading style.
//! **Position:** inside the head row of the combat history, the caller's leave table and the
//! administrator's review queue.
//! **Signals & state:** none.
//! **Invariants:** the label is a static string, so a heading is never built from wire data.
#![allow(dead_code)]

use leptos::prelude::*;

/// One table column heading.
#[component]
pub(super) fn ServiceHead(label: &'static str) -> impl IntoView {
    view! {
        <th class="px-4 py-3 font-mono text-[10px] font-normal tracking-widest text-on-surface-variant uppercase">
            {label}
        </th>
    }
}
