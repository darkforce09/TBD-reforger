//! The master-detail layout, and the pieces the pages that use it share.
//!
//! **Role:** a fixed-width list beside a flexible detail pane, plus the row, the filter field, the
//! empty state and the match predicate that go with it.
//! **Position:** fills a page's content area. The detail pane is a nested main element inside the
//! shell's own.
//! **Signals & state:** none of its own — the filter field drives a signal the caller owns, and
//! selection is the caller's to hold.
//! **Invariants:** slot content is passed as already-built views rather than as closures, so a
//! caller composes the two panes itself and this owns only the frame between them.
#![allow(dead_code)]
use crate::v2::core::ui::{cn, MaterialIcon};
use leptos::prelude::*;

/// A fixed-width list beside a flexible detail pane.
///
/// Renders the master column — with `master_header` above it when given — beside the detail pane.
/// `transparent` drops the patterned background for callers that supply their own. Fills whatever
/// content area it is placed in.
#[component]
pub fn SplitPane(
    master: AnyView,
    detail: AnyView,
    #[prop(optional)] master_header: Option<AnyView>,
    #[prop(default = "22rem")] master_width: &'static str,
    #[prop(optional)] transparent: bool,
) -> impl IntoView {
    let outer = cn(&[
        "flex h-full min-h-0 w-full overflow-hidden",
        // Only the grid overlay survives: two background utilities on one element collapse to the
        // last one declared.
        if transparent { "" } else { "bg-grid-overlay" },
    ]);
    view! {
        <div class=outer>
            <aside
                class="flex h-full min-h-0 shrink-0 flex-col border-r border-outline-variant/30 bg-surface-container-lowest/50"
                style=format!("width: {master_width}; max-width: 90vw;")
            >
                {master_header
                    .map(|h| {
                        view! {
                            <div class="flex shrink-0 items-center justify-between gap-2 border-b border-outline-variant/30 px-4 py-3">
                                {h}
                            </div>
                        }
                    })}
                <div class="custom-scrollbar flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto p-3">
                    {master}
                </div>
            </aside>
            <main class="custom-scrollbar relative flex h-full min-h-0 flex-1 flex-col overflow-y-auto bg-surface-container-highest/10">
                {detail}
            </main>
        </div>
    }
}

/// The detail pane's placeholder, shown while nothing is selected.
///
/// Renders a centred glyph and line in place of the detail content.
#[component]
pub fn SplitPaneEmpty(
    #[prop(optional)] icon: Option<AnyView>,
    message: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex h-full flex-col items-center justify-center gap-3 text-outline">
            {icon}
            <p class="text-label-md">{message}</p>
        </div>
    }
}

/// The reference pages' wrapper: a patterned background, a frosted panel, and a transparent
/// [`SplitPane`] inside it.
///
/// Renders the three layers in that order and fills the page. Used by the pages that present
/// reference material rather than live data.
#[component]
pub fn GlassSplit(
    master_header: AnyView,
    master: AnyView,
    detail: AnyView,
    #[prop(optional)] master_width: Option<&'static str>,
) -> impl IntoView {
    let mw = master_width.unwrap_or("22rem");
    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                <SplitPane
                    transparent=true
                    master_width=mw
                    master_header=master_header
                    master=master
                    detail=detail
                />
            </div>
        </div>
    }
}

/// The filter field above a master list.
///
/// Renders one input with a leading glyph. Pass `bind` to make it live: the input writes that
/// signal and the caller filters on it.
#[component]
pub fn SidebarSearch(
    #[prop(optional)] value: &'static str,
    placeholder: &'static str,
    #[prop(optional)] bind: Option<RwSignal<String>>,
) -> impl IntoView {
    view! {
           <div class="relative w-full">
               <MaterialIcon
                   name="search"
                   class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-base text-on-surface-variant"
               />
    // Uncontrolled: the signal mirrors the field rather than driving it, so the rendered
    // attributes are the same whether or not a caller binds one.
               <input
                   type="search"
                   value=value
                   placeholder=placeholder
                   on:input=move |ev| {
                       if let Some(b) = bind {
                           b.set(event_target_value(&ev));
                       }
                   }
                   class="w-full rounded-lg border border-white/10 bg-black/30 py-2 pr-3 pl-9 text-sm text-on-surface placeholder:text-on-surface-variant/60 focus:border-primary/50 focus:outline-none"
               />
           </div>
       }
}

/// Case-insensitive substring match against a haystack built from a row's searchable fields.
///
/// An empty or whitespace-only query matches everything, which is the shared contract the pages
/// that filter with this rely on.
pub fn search_matches(query: &str, haystack: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || haystack.to_lowercase().contains(&q)
}

#[cfg(test)]
#[path = "tests/split_pane.rs"]
mod tests;

/// One row of a master list.
///
/// Renders a button carrying the title and subtitle, with optional leading preview and trailing
/// content, and a pulse dot when `pulse` is set. Placed inside a [`SplitPane`]'s master column.
#[component]
pub fn ListDetailItem(
    title: AnyView,
    #[prop(optional)] active: bool,
    #[prop(optional)] meta: Option<AnyView>,
    #[prop(optional)] dot_class: &'static str,
    #[prop(optional)] pulse: bool,
    #[prop(optional)] preview: Option<AnyView>,
    #[prop(optional)] trailing: Option<AnyView>,
    #[prop(optional)] class: &'static str,
    /// Fired when the row is chosen.
    #[prop(optional)]
    on_click: Option<Callback<()>>,
) -> impl IntoView {
    let btn = cn(&[
        "group relative w-full overflow-hidden rounded-lg border p-3 text-left transition-all duration-200",
        if active {
            "border-primary/30 bg-surface-variant/80 shadow-[inset_0_0_15px_rgba(173,198,255,0.1)]"
        } else {
            "border-transparent hover:border-outline-variant/30 hover:bg-surface-variant/40"
        },
        class,
    ]);
    let title_class = if active {
        "truncate font-semibold text-on-surface"
    } else {
        "truncate font-semibold text-on-surface-variant group-hover:text-on-surface"
    };
    // cn: the custom `text-code-md` is twMerge-dropped against the trailing text-{color}.
    let meta_class = cn(&[
        "font-mono",
        if active {
            "text-primary opacity-80"
        } else {
            "text-outline"
        },
    ]);
    let has_meta = meta.is_some() || !dot_class.is_empty();
    let dot_class_full = cn(&[
        "mt-1 h-2 w-2 shrink-0 rounded-full",
        dot_class,
        if pulse { "animate-pulse" } else { "" },
    ]);
    view! {
        <button
            type="button"
            class=btn
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            {active.then(|| view! { <span class="absolute top-0 bottom-0 left-0 w-1 bg-primary"></span> })}
            {has_meta
                .then(move || {
                    view! {
                        <div class="mb-1 flex items-start justify-between gap-2">
                            {meta.map(|m| view! { <span class=meta_class.clone()>{m}</span> })}
                            {(!dot_class.is_empty())
                                .then(|| view! { <span class=dot_class_full.clone()></span> })}
                        </div>
                    }
                })}
            <div class="flex items-center justify-between gap-2">
                <h3 class=title_class>{title}</h3>
                {trailing}
            </div>
            {preview
                .map(|p| {
                    view! {
                        <p class="mt-1.5 line-clamp-2 text-label-sm text-outline normal-case">{p}</p>
                    }
                })}
        </button>
    }
}
