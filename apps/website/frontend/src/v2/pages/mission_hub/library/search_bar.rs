//! The free-text search box above the mission grid.
//!
//! **Role:** one control — the query the mission list is filtered by.
//! **Position:** the first item in the library's search and filter toolbar.
//! **Signals & state:** writes the page's `q` signal on every keystroke; the list resource
//! re-keys on it, so typing refetches.
//! **Invariants:** the rest-state `value` attribute is empty and the live binding is the
//! property, which is the shape the captured markup is pinned against.

use leptos::prelude::*;

/// The search field, bound to the page's query signal.
pub(super) fn search_bar(q: RwSignal<String>) -> impl IntoView {
    view! {
                <input
                    type="search"
                    placeholder="Search operations..."
                    // The rest-state attribute is empty; the live binding is the property below.
                    value=""
                    prop:value=move || q.get()
                    on:input=move |ev| q.set(event_target_value(&ev))
                    class="min-w-[200px] flex-1 rounded-lg border border-white/10 bg-black/30 px-4 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60"
                />
    }
}
