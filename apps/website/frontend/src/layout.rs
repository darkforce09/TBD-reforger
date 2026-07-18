//! Platform shell — ported from components/layout/{AppLayout,Sidebar,TopNav}.tsx. DOM structure +
//! class strings matched 1:1 to the React output (V-shell gate, byte-equal). Auth (role, user
//! menu, breadcrumb source) and routing are stubbed to the guest "/" render until T-159.3 / .4.
use crate::app_routes::AppRoutes;
use crate::auth::AuthStore;
use crate::nav::{has_min_role, NavItem, Role, NAVIGATION};
use crate::ui::{cn, MaterialIcon, DEFAULT_AVATAR};
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

/// Which shell frame a pathname gets. Derived in a `Memo` so SPA navigation only remounts the
/// frame when the *kind* changes (login ↔ chrome ↔ editor), not on every route (T-172 A2/A8).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    /// `/login` + `/auth/callback` — React renders these outside AppLayout, bare.
    Bare,
    /// Mission Creator editor — full-viewport, no platform chrome.
    Chromeless,
    /// Sidebar + TopNav + `<main>`.
    Chrome,
}

fn classify_frame(path: &str) -> FrameKind {
    if path == "/login" || path == "/auth/callback" {
        FrameKind::Bare
    } else if crate::router::chromeless(path) {
        FrameKind::Chromeless
    } else {
        FrameKind::Chrome
    }
}

#[component]
pub fn AppLayout() -> impl IntoView {
    // The auth store (Zustand replacement) lives at the shell root; children read it via context.
    // Cold-load bootstrap (refresh from tbd-auth) + the gloo-net client populate it next.
    provide_context(AuthStore::new());
    // Toasts context (sonner parity — React mounts <Toaster/> once in main.tsx). T-159.25.
    crate::toast::provide_toasts();
    // Cold-load bootstrap: hydrate the session from tbd-auth (no-op for a guest with nothing stored).
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(crate::client::bootstrap(expect_context::<AuthStore>()));
    // Route determines the frame — reactive on SPA nav (T-172 A2/A8). The Memo dedups by
    // FrameKind, so navigating between two Chrome routes never remounts Sidebar/TopNav; only
    // crossing a login/editor boundary swaps the frame.
    let pathname = use_location().pathname;
    let frame_kind = Memo::new(move |_| classify_frame(&pathname.get()));
    let frame = move || match frame_kind.get() {
        // React renders these OUTSIDE AppLayout — bare, no chrome, no wrapper div.
        FrameKind::Bare => view! { <AppRoutes /> }.into_any(),
        // The Mission Creator editor: AppLayout's chromeless full-viewport branch.
        FrameKind::Chromeless => view! {
            <div class="h-screen w-screen overflow-hidden bg-background">
                <AppRoutes />
            </div>
        }
        .into_any(),
        // Normal chrome: Sidebar + TopNav + the padded/full-bleed <main>.
        FrameKind::Chrome => {
            let main_class = move || {
                if crate::router::full_bleed(&pathname.get()) {
                    "min-h-0 flex-1 bg-background overflow-hidden"
                } else {
                    "min-h-0 flex-1 bg-background overflow-y-auto p-6"
                }
            };
            // Narrow-viewport slide-over nav (T-172 A9). No DOM while closed.
            let mobile_open = RwSignal::new(false);
            #[cfg(target_arch = "wasm32")]
            {
                let esc = window_event_listener(leptos::ev::keydown, move |ev| {
                    if mobile_open.get_untracked() && ev.key() == "Escape" {
                        mobile_open.set(false);
                    }
                });
                on_cleanup(move || esc.remove());
            }
            view! {
                <div class="flex h-screen overflow-hidden bg-background">
                    <SidebarMobileToggle open=mobile_open />
                    {move || {
                        mobile_open.get().then(|| view! {
                            <div
                                class="animate-overlay-fade fixed inset-0 z-40 bg-black/50 lg:hidden"
                                on:click=move |_| mobile_open.set(false)
                            ></div>
                            <aside class="animate-sheet-in-left fixed inset-y-0 left-0 z-50 flex w-80 flex-col bg-surface-container-low lg:hidden">
                                <SidebarBrand />
                                <SidebarNav on_nav=Callback::new(move |()| mobile_open.set(false)) />
                            </aside>
                        })
                    }}
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
    };
    // The viewport is a sibling of the frame (like React's root-level <Toaster/>) and renders no
    // DOM while the toast list is empty, so byte-equal V captures are unaffected.
    view! {
        {frame}
        <crate::toast::ToastViewport />
    }
}

#[component]
fn TopNav() -> impl IntoView {
    // Breadcrumb from the live route — reactive on SPA nav (T-172 A8).
    let pathname = use_location().pathname;
    let auth = expect_context::<AuthStore>();
    // User-menu dropdown (T-172 A1). Renders no DOM while closed (V-suite byte-equal).
    let menu_open = RwSignal::new(false);
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if menu_open.get_untracked() && ev.key() == "Escape" {
                menu_open.set(false);
            }
        });
        on_cleanup(move || esc.remove());
    }
    // Sign Out: revoke the presented refresh token server-side (fire-and-forget — logout always
    // 204s), then drop + persist the cleared session so a reload can't resurrect it.
    let sign_out = move |_| {
        menu_open.set(false);
        #[cfg(target_arch = "wasm32")]
        {
            let rt = auth.refresh_token.get_untracked();
            auth.clear_session();
            crate::auth::persist(&auth.persist_state());
            leptos::task::spawn_local(async move {
                if let Some(rt) = rt {
                    let _ = crate::client::api_post_ok(
                        auth,
                        "/auth/logout",
                        serde_json::json!({ "refresh_token": rt }),
                    )
                    .await;
                }
            });
        }
    };
    view! {
        <header class="flex h-16 shrink-0 items-center justify-between border-b border-outline-variant/30 bg-surface-container-low/70 px-6 backdrop-blur-xl">
            <div class="flex h-full min-w-0 items-center gap-2 pl-12 lg:pl-0">
                {move || match crate::router::breadcrumb(&pathname.get()) {
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
            // Reactive: guest sign-in CTA until bootstrap lands the session, then StatusPill + the
            // avatar button with its dropdown (Settings / Link Arma Identity / Sign Out).
            <div class="relative flex h-full items-center gap-4">
                {move || {
                    if !auth.is_authenticated() {
                        return view! {
                            <a
                                href="/login"
                                class="rounded-lg bg-primary px-4 py-2 text-label-md font-medium text-on-primary"
                            >
                                "Sign in with Discord"
                            </a>
                        }
                            .into_any();
                    }
                    let user = auth.user.get();
                    let username = user.as_ref().map(|u| u.username.clone()).unwrap_or_default();
                    let avatar = user
                        .as_ref()
                        .map(|u| u.avatar_url.clone())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| DEFAULT_AVATAR.to_string());
                    // StatusPill: linked iff arma_id is present/non-empty.
                    let arma_id = user.as_ref().and_then(|u| u.arma_id.clone()).filter(|s| !s.is_empty());
                    let pill = match arma_id {
                        Some(id) => {
                            let short: String = id.chars().take(8).collect();
                            view! {
                                <div class="rounded-full bg-success-muted px-3 py-1 font-mono text-xs text-success">
                                    "Linked: "
                                    {short}
                                    "..."
                                </div>
                            }
                                .into_any()
                        }
                        None => view! {
                            <div class="rounded-full bg-surface-container-high px-3 py-1 text-xs text-on-surface-variant">
                                "Unlinked"
                            </div>
                        }
                            .into_any(),
                    };
                    view! {
                        <>
                            {pill}
                            <button
                                type="button"
                                class="flex items-center gap-2 rounded-lg p-1 pr-3 transition-colors hover:bg-surface-variant/50"
                                on:click=move |_| menu_open.update(|v| *v = !*v)
                            >
                                <img
                                    src=avatar
                                    alt=""
                                    class="h-8 w-8 rounded-full border border-outline-variant/50 object-cover"
                                />
                                <span class="text-label-md font-medium">{username}</span>
                                <MaterialIcon name="expand_more" class="text-on-surface-variant" />
                            </button>
                            {move || {
                                menu_open.get().then(|| {
                                    let item = "flex w-full items-center gap-2 px-4 py-2 text-left text-label-md text-on-surface transition-colors hover:bg-surface-variant/40";
                                    view! {
                                        <div class="fixed inset-0 z-40" on:click=move |_| menu_open.set(false)></div>
                                        <div class="glass animate-dialog-in absolute top-full right-0 z-50 mt-2 w-56 rounded-lg py-1 shadow-lg">
                                            <a href="/settings" class=item on:click=move |_| menu_open.set(false)>
                                                <MaterialIcon name="settings" class="text-[18px] text-on-surface-variant" />
                                                "Settings"
                                            </a>
                                            <a
                                                href="/settings#arma-link"
                                                class=item
                                                on:click=move |_| menu_open.set(false)
                                            >
                                                <MaterialIcon name="link" class="text-[18px] text-on-surface-variant" />
                                                "Link Arma Identity"
                                            </a>
                                            <hr class="my-1 border-outline-variant/30" />
                                            <button
                                                type="button"
                                                class="flex w-full items-center gap-2 px-4 py-2 text-left text-label-md text-error transition-colors hover:bg-error/10"
                                                on:click=sign_out
                                            >
                                                <MaterialIcon name="logout" class="text-[18px]" />
                                                "Sign Out"
                                            </button>
                                        </div>
                                    }
                                })
                            }}
                        </>
                    }
                        .into_any()
                }}
            </div>
        </header>
    }
}

#[component]
fn SidebarMobileToggle(open: RwSignal<bool>) -> impl IntoView {
    // Toggles the narrow-viewport slide-over (T-172 A9). Button markup unchanged (V gate);
    // hidden ≥lg via lg:hidden.
    view! {
        <button
            type="button"
            class="fixed top-3 left-3 z-50 rounded-md bg-surface-container p-2 lg:hidden"
            aria-label="Open menu"
            on:click=move |_| open.update(|v| *v = !*v)
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
fn SidebarNav(
    /// Invoked on any nav-link click — the mobile drawer closes itself through this (T-172 A9).
    #[prop(optional)]
    on_nav: Option<Callback<()>>,
) -> impl IntoView {
    // Real auth store (guest → None → browse-mode, all nav) + the live route for the active link.
    // Both are read inside one reactive closure (T-172 A2 + H1): the active highlight follows SPA
    // navigation and the admin section appears/disappears with the session, no reload needed. The
    // rendered class strings are byte-identical to the one-shot version (V gate).
    let auth = expect_context::<AuthStore>();
    let pathname = use_location().pathname;
    view! {
        <nav class="custom-scrollbar flex-1 overflow-y-auto px-3 py-4">
            {move || {
                let user_role = auth.user.get().map(|u| u.role);
                let current = pathname.get();
                NAVIGATION
                .iter()
                .filter_map(|section| {
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
                                                    on:click=move |_| {
                                                        if let Some(cb) = on_nav {
                                                            cb.run(());
                                                        }
                                                    }
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
                .collect_view()
            }}
        </nav>
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_frame, is_active, FrameKind};

    #[test]
    fn is_active_dashboard_exact() {
        assert!(is_active("/", "/"));
        assert!(!is_active("/", "/missions"));
    }

    #[test]
    fn is_active_prefix_and_exact() {
        assert!(is_active("/missions", "/missions"));
        assert!(is_active("/missions", "/missions/abc"));
        assert!(!is_active("/missions", "/missions-archive"));
        assert!(!is_active("/events", "/missions"));
    }

    #[test]
    fn classify_frame_kinds() {
        assert!(matches!(classify_frame("/login"), FrameKind::Bare));
        assert!(matches!(classify_frame("/auth/callback"), FrameKind::Bare));
        assert!(matches!(
            classify_frame("/missions/abc/edit"),
            FrameKind::Chromeless
        ));
        assert!(matches!(classify_frame("/"), FrameKind::Chrome));
        assert!(matches!(classify_frame("/missions"), FrameKind::Chrome));
    }
}
