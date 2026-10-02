//! The library's page header: the title, the scope tabs and the New Mission button.
//!
//! **Role:** names the page, offers the three scopes a mission list can be drawn from, and — for
//! anyone allowed to create one — the button that opens the create dialog.
//! **Position:** the top of the `/missions` route, above the body.
//! **Signals & state:** writes the page's `scope_idx` signal; reads the page's maker memo to
//! decide whether the create button appears.
//! **Invariants:** the scope order here is the order [`SCOPES`] declares, and the index the page
//! holds is an index into it, so the two can never disagree about which tab is selected.

use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// The three scopes the mission list can be drawn from, as `(tab label, query value)`.
///
/// The page holds an index into this array, so the tab order and the query values stay one
/// declaration.
pub(super) const SCOPES: [(&str, &str); 3] = [
    ("Global Missions", "global"),
    ("My Missions", "mine"),
    ("Bookmarked", "bookmarked"),
];

/// The page header, with the scope tabs and the create affordance.
///
/// `open_create` closes the dossier sheet before opening the dialog, so only one overlay is ever
/// on screen.
pub(super) fn library_header(
    is_maker: Memo<bool>,
    scope_idx: RwSignal<usize>,
    open_create: impl Fn() + Copy + Send + 'static,
) -> impl IntoView {
    view! {
        <header class="mb-6 flex flex-wrap items-start justify-between gap-4">
            <div>
                <h1 class="text-4xl font-bold tracking-tight text-on-surface uppercase">
                    "Mission Library"
                </h1>
                <p class="mt-1 text-body-md text-on-surface-variant">
                    "Browse, filter, and deploy active operations across the theater."
                </p>
                <div class="mt-5 inline-flex gap-1 rounded-full border border-white/5 bg-black/20 p-1">
                    {SCOPES
                        .iter()
                        .enumerate()
                        .map(|(i, (label, _))| {
                            // The utility merge drops the base text size against the trailing
                            // colour class, so the size is not restated in either arm.
                            view! {
                                <button
                                    type="button"
                                    on:click=move |_| scope_idx.set(i)
                                    class=move || {
                                        if scope_idx.get() == i {
                                            "rounded-full px-4 py-1.5 font-medium transition-all bg-surface-glass text-on-surface shadow-md"
                                        } else {
                                            "rounded-full px-4 py-1.5 font-medium transition-all text-on-surface-variant hover:text-on-surface"
                                        }
                                    }
                                >
                                    {*label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
            {move || {
                is_maker.get().then(|| {
                    view! {
                        <button
                            type="button"
                            on:click=move |_| open_create()
                            title="New Mission (Ctrl+N)"
                            class="flex items-center gap-2 rounded-full bg-action px-6 py-3 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                        >
                            <MaterialIcon name="add" class="text-[18px]" />
                            "New Mission"
                        </button>
                    }
                })
            }}
        </header>
    }
}
