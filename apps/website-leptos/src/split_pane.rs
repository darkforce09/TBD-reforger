//! SplitPane — the Apple-Mail / Finder master-detail layout. Ported from
//! components/ui/split-pane.tsx. Reusable across Announcements, Deployments, doctrine, etc. Slot
//! content (master / detail / master_header) is passed as `AnyView` props, mirroring the React
//! `ReactNode` props. Note: the detail pane is a nested `<main>` (inside AppLayout's `<main>`).
#![allow(dead_code)]
use crate::ui::cn;
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
        if transparent {
            ""
        } else {
            "bg-topo-map bg-grid-overlay"
        },
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
