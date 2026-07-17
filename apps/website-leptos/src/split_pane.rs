//! SplitPane — the Apple-Mail / Finder master-detail layout. Ported from
//! components/ui/split-pane.tsx. Reusable across Announcements, Deployments, doctrine, etc. Slot
//! content (master / detail / master_header) is passed as `AnyView` props, mirroring the React
//! `ReactNode` props. Note: the detail pane is a nested `<main>` (inside AppLayout's `<main>`).
#![allow(dead_code)]
use crate::ui::{cn, MaterialIcon};
use leptos::prelude::*;

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
        // React's cn(…, !transparent && 'bg-topo-map bg-grid-overlay') tailwind-merges the two bg-*
        // utilities and keeps only the last → `bg-grid-overlay` (bg-topo-map is dropped).
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

/// Placeholder shown in the detail pane when nothing is selected.
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

/// The doctrine wrapper: topo-map background → frosted glass → a transparent SplitPane. Ported from
/// doctrine.tsx `GlassSplit` (reused by Modpacks / Wiki / Vehicle Database).
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

/// Search box used across the doctrine master panes. Ported from doctrine.tsx `SidebarSearch`.
#[component]
pub fn SidebarSearch(
    #[prop(optional)] value: &'static str,
    placeholder: &'static str,
) -> impl IntoView {
    view! {
        <div class="relative w-full">
            <MaterialIcon
                name="search"
                class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-base text-on-surface-variant"
            />
            <input
                type="search"
                value=value
                placeholder=placeholder
                class="w-full rounded-lg border border-white/10 bg-black/30 py-2 pr-3 pl-9 text-sm text-on-surface placeholder:text-on-surface-variant/60 focus:border-primary/50 focus:outline-none"
            />
        </div>
    }
}

/// Recurring master-list row for SplitPane left panes. Ported from components/ui/list-detail-item.tsx.
/// (title h3 cn() drops the base `text-label-md` vs the trailing text-{color}, per twMerge.)
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
    /// T-159.25 — selection wiring (React rows pass onClick).
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
    // cn(): the custom `text-code-md` is twMerge-dropped against the trailing text-{color}.
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
