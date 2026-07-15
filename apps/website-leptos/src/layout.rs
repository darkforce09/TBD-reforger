//! Platform shell — ported from components/layout/{AppLayout,Sidebar,TopNav}.tsx. DOM structure +
//! class strings matched 1:1 to the React output (V-shell gate, byte-equal). Auth (role, user
//! menu, breadcrumb source) and routing are stubbed to the guest "/" render until T-159.3 / .4.
use crate::app_routes::AppRoutes;
use crate::auth::AuthStore;
use crate::nav::{has_min_role, NavItem, Role, NAVIGATION};
use crate::ui::{cn, MaterialIcon};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// NavLink active matching: Dashboard ("/") is exact (`end`); every other link is active on an
/// exact match or a sub-path prefix — mirrors react-router's NavLink.
fn is_active(path: &str, current: &str) -> bool {
    if path == "/" {
        current == "/"
    } else {
        current == path || current.starts_with(&format!("{path}/"))
    }
}

#[component]
pub fn AppLayout() -> impl IntoView {
    // The auth store (Zustand replacement) lives at the shell root; children read it via context.
    // Cold-load bootstrap (refresh from tbd-auth) + the gloo-net client populate it next.
    provide_context(AuthStore::new());
    // Route determines the frame. Read once at load; reactive re-wrap on SPA nav is a follow-up.
    let path = use_location().pathname.get();
    if path == "/login" || path == "/auth/callback" {
        // React renders these OUTSIDE AppLayout — bare, no chrome, no wrapper div.
        view! { <AppRoutes /> }.into_any()
    } else if crate::router::chromeless(&path) {
        // The Mission Creator editor: AppLayout's chromeless full-viewport branch.
        view! {
            <div class="h-screen w-screen overflow-hidden bg-background">
                <AppRoutes />
            </div>
        }
        .into_any()
    } else {
        // Normal chrome: Sidebar + TopNav + the padded/full-bleed <main>.
        let main_class = if crate::router::full_bleed(&path) {
            "min-h-0 flex-1 bg-background overflow-hidden"
        } else {
            "min-h-0 flex-1 bg-background overflow-y-auto p-6"
        };
        view! {
            <div class="flex h-screen overflow-hidden bg-background">
                <SidebarMobileToggle />
                <Sidebar />
                <div class="flex min-w-0 flex-1 flex-col">
                    <TopNav />
                    <main class=main_class>
                        <AppRoutes />
                    </main>
                </div>
            </div>
        }
        .into_any()
    }
}

#[component]
fn TopNav() -> impl IntoView {
    // Breadcrumb from the live route (exact-match; dynamic-route patterns are a follow-up). Guest
    // auth state until the gloo-net bootstrap lands.
    let breadcrumb = crate::router::breadcrumb(&use_location().pathname.get());
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
    // Real auth store (guest → None → browse-mode, all nav) + the live route for the active link.
    let user_role = expect_context::<AuthStore>().user.get().map(|u| u.role);
    // Read once at render → correct at page load (what the V gate checks). Wrapping the class in a
    // reactive closure so active follows SPA navigation without a reload is a small follow-up.
    let current = use_location().pathname.get();
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
                                        let active = is_active(item.path, &current);
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
