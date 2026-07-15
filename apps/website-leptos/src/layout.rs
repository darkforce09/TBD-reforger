//! Platform shell — ported from components/layout/{AppLayout,Sidebar,TopNav}.tsx. DOM structure +
//! class strings matched 1:1 to the React output (V-shell gate, byte-equal). Auth (role, user
//! menu, breadcrumb source) and routing are stubbed to the guest "/" render until T-159.3 / .4.
use crate::auth::AuthStore;
use crate::nav::{has_min_role, NavItem, Role, NAVIGATION};
use crate::ui::{cn, MaterialIcon};
use leptos::prelude::*;

#[component]
pub fn AppLayout() -> impl IntoView {
    // The auth store (Zustand replacement) lives at the shell root; children read it via context.
    // Cold-load bootstrap (refresh from tbd-auth) + the gloo-net client populate it next.
    provide_context(AuthStore::new());
    // The "/" route: guest, non-chromeless, fullBleed → <main> is overflow-hidden.
    view! {
        <div class="flex h-screen overflow-hidden bg-background">
            <SidebarMobileToggle />
            <Sidebar />
            <div class="flex min-w-0 flex-1 flex-col">
                <TopNav />
                <main class="min-h-0 flex-1 bg-background overflow-hidden"></main>
            </div>
        </div>
    }
}

#[component]
fn TopNav() -> impl IntoView {
    // Stubs until T-159.3 (auth) / T-159.4 (router): the guest state + the "/" breadcrumb.
    let breadcrumb: Option<(&str, &str)> = Some(("Command Center", "Dashboard"));
    let is_authenticated = expect_context::<AuthStore>().is_authenticated();
    view! {
        <header class="flex h-16 shrink-0 items-center justify-between border-b border-outline-variant/30 bg-surface-container-low/70 px-6 backdrop-blur-xl">
            <div class="flex h-full min-w-0 items-center gap-2 pl-12 lg:pl-0">
                {match breadcrumb {
                    Some((parent, current)) => view! {
                        <>
                            <span class="text-label-md text-on-surface-variant">{parent}</span>
                            <span class="text-outline">"/"</span>
                            <span class="text-label-md font-semibold text-on-surface">{current}</span>
                        </>
                    }
                        .into_any(),
                    None => view! {
                        <span class="text-label-md font-semibold text-on-surface">"TBD Reforger"</span>
                    }
                        .into_any(),
                }}
            </div>
            <div class="relative flex h-full items-center gap-4">
                {if !is_authenticated {
                    view! {
                        <a
                            href="/login"
                            class="rounded-lg bg-primary px-4 py-2 text-label-md font-medium text-on-primary"
                        >
                            "Sign in with Discord"
                        </a>
                    }
                        .into_any()
                } else {
                    // Authed: StatusPill + avatar menu — ported with the auth store (T-159.3).
                    ().into_any()
                }}
            </div>
        </header>
    }
}

#[component]
fn SidebarMobileToggle() -> impl IntoView {
    // Closed by default (mobileOpen = false): just the toggle button (hidden ≥lg via lg:hidden).
    // The slide-over overlay is interactive state for a later slice.
    view! {
        <button
            type="button"
            class="fixed top-3 left-3 z-50 rounded-md bg-surface-container p-2 lg:hidden"
            aria-label="Open menu"
        >
            <MaterialIcon name="menu" />
        </button>
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
    // Real auth store (guest → None → browse-mode, all nav). T-159.4 wires the active route.
    let user_role = expect_context::<AuthStore>().user.get().map(|u| u.role);
    let current = "/";
    view! {
        <nav class="custom-scrollbar flex-1 overflow-y-auto px-3 py-4">
            {NAVIGATION
                .iter()
                .filter_map(move |section| {
                    if section.admin && !has_min_role(user_role, Role::Admin) {
                        return None;
                    }
                    let items: Vec<&NavItem> =
                        section.items.iter().filter(|i| has_min_role(user_role, i.min_role)).collect();
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
                                        // React's cn (tailwind-merge) DROPS `text-label-md` here: it
                                        // collides with the trailing text-{color} and twMerge keeps
                                        // the last, so the rendered link inherits 16px. We omit it to
                                        // match byte-for-byte (a general Rust tw_merge is a follow-up).
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
