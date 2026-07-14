//! Platform shell — ported from components/layout/{AppLayout,Sidebar}.tsx. The DOM structure and
//! class strings are matched 1:1 to the React output (V-shell gate). TopNav + mobile toggle land
//! next; auth (role) and the active route are stubbed to guest / "/" until T-159.3 / T-159.4.
use crate::nav::{has_min_role, NavItem, Role, NAVIGATION};
use crate::ui::{cn, MaterialIcon};
use leptos::prelude::*;

#[component]
pub fn AppLayout() -> impl IntoView {
    view! {
        <div class="flex h-screen overflow-hidden bg-background">
            <Sidebar />
            <div class="flex min-w-0 flex-1 flex-col">
                // TopNav + padded/full-bleed <main> land in the next shell slice.
                <main class="min-h-0 flex-1 overflow-hidden bg-background"></main>
            </div>
        </div>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    view! {
        <aside class="hidden h-screen w-80 shrink-0 flex-col bg-surface-container-low lg:flex">
            <SidebarBrand />
            <SidebarNav />
        </aside>
    }
}

#[component]
fn SidebarBrand() -> impl IntoView {
    view! {
        <header class="relative flex h-16 shrink-0 items-center px-6">
            <div class="flex items-center gap-2">
                <span class="text-2xl font-bold tracking-wide text-primary">"TBD"</span>
                <span class="text-2xl font-bold tracking-wide text-on-surface">"Reforger"</span>
            </div>
        </header>
    }
}

#[component]
fn SidebarNav() -> impl IntoView {
    let user: Option<Role> = None; // T-159.3: wire to the real auth store.
    let current = "/"; // T-159.4: wire to the real router location.
    view! {
        <nav class="custom-scrollbar flex-1 overflow-y-auto px-3 py-4">
            {NAVIGATION
                .iter()
                .filter_map(move |section| {
                    if section.admin && !has_min_role(user, Role::Admin) {
                        return None;
                    }
                    let items: Vec<&NavItem> =
                        section.items.iter().filter(|i| has_min_role(user, i.min_role)).collect();
                    if items.is_empty() {
                        return None;
                    }
                    let section_class = cn(&[
                        "mb-6",
                        if section.admin { "rounded-lg border border-red-500/20 bg-red-900/10 p-3" } else { "" },
                    ]);
                    let h3_class = cn(&[
                        "mb-2 px-2 text-xs font-bold tracking-widest uppercase",
                        if section.admin { "text-red-400" } else { "text-gray-500" },
                    ]);
                    Some(view! {
                        <div class=section_class>
                            <h3 class=h3_class>{section.title}</h3>
                            <ul class="space-y-1">
                                {items
                                    .into_iter()
                                    .map(|item| {
                                        let active = item.path == current;
                                        // React's cn (tailwind-merge) DROPS `text-label-md` here:
                                        // it collides with the trailing text-{color} and twMerge
                                        // keeps the last, so the rendered link inherits 16px. We
                                        // omit it to match that exact output byte-for-byte (a
                                        // general Rust tw_merge is a separate migration-wide task).
                                        let a_class = cn(&[
                                            "flex items-center gap-3 rounded-md px-3 py-2.5 font-medium transition-colors",
                                            if active {
                                                "nav-item-active text-primary"
                                            } else {
                                                "text-on-surface-variant hover:bg-surface-variant/40 hover:text-on-surface"
                                            },
                                        ]);
                                        view! {
                                            <li>
                                                <a
                                                    href=item.path
                                                    class=a_class
                                                    aria-current=active.then_some("page")
                                                >
                                                    <MaterialIcon name=item.icon class="text-[22px]" />
                                                    {item.label}
                                                </a>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        </div>
                    })
                })
                .collect_view()}
        </nav>
    }
}
