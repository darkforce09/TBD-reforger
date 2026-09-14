//! The frame that wraps every page, and the rule that decides which frame a route gets.
//!
//! **Role:** owns [`AppLayout`] — the component mounted once at the application root — together
//! with the pathname classifier behind it and the active-link rule the sidebar renders with.
//! **Position:** outside the router's outlet. The router swaps `<main>`; this frame does not
//! remount unless the *kind* of frame changes.
//! **Signals & state:** provides the session store and the toast queue as context. Reads the live
//! pathname, derives [`FrameKind`] from it through a `Memo`, and owns the mobile drawer's
//! open/closed signal, which the drawer's backdrop, its toggle and the escape key all write.
//! **Invariants:** there are exactly three frames, and [`classify_frame`] is the only place the
//! choice is made:
//!
//! * **Bare** — `/login` and `/auth/callback`: the outlet alone, no wrapper element.
//! * **Chromeless** — every route the route table marks `chromeless`, today the mission editor:
//!   a `h-screen w-screen overflow-hidden` container and nothing else.
//! * **Chrome** — everything else: the sidebar, the top bar, and a `<main>` that is
//!   `overflow-hidden` for a route the table marks `full_bleed` and a padded scroll container
//!   otherwise.
//!
//! The last two mirror the route table directly: the chromeless branch asks
//! [`crate::router::chromeless`] and the `<main>` class asks [`crate::router::full_bleed`], so a
//! route's layout is declared once, in the table, and never restated here. The bare branch is the
//! exception — those two paths are named in this file because they are the frame's own boundary,
//! not a route-table flag.

use crate::app_routes::AppRoutes;
use crate::v2::core::auth::AuthStore;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::sidebar::{Sidebar, SidebarBrand, SidebarMobileToggle, SidebarNav};
use super::top_nav::TopNav;

/// Whether the link to `path` should render as the active one for the pathname `current`.
///
/// The dashboard link (`"/"`) matches exactly, since every path starts with it. Every other link
/// matches its own path and any path below it, so `/missions/abc` keeps Mission Library
/// highlighted while `/missions-archive` does not.
pub(super) fn is_active(path: &str, current: &str) -> bool {
    if path == "/" {
        current == "/"
    } else {
        current == path || current.starts_with(&format!("{path}/"))
    }
}

/// Which of the three frames a pathname gets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    /// The sign-in pages: the outlet with no wrapper at all.
    Bare,
    /// A route that takes the whole viewport with no platform chrome.
    Chromeless,
    /// The standard frame: sidebar, top bar and `<main>`.
    Chrome,
}

/// Classify a pathname into its frame.
///
/// The sign-in paths are named here; everything else defers to the route table, so adding a
/// full-viewport route is a table edit rather than a change to this rule.
fn classify_frame(path: &str) -> FrameKind {
    if path == "/login" || path == "/auth/callback" {
        FrameKind::Bare
    } else if crate::router::chromeless(path) {
        FrameKind::Chromeless
    } else {
        FrameKind::Chrome
    }
}

/// The application frame: one of three shapes, chosen by the route.
///
/// Renders the sign-in shape bare, the editor shape as a full-viewport container, and everything
/// else as sidebar + top bar + `<main>`. Provides the session store and the toast queue to
/// everything below, and mounts the toast viewport beside the frame.
#[component]
pub fn AppLayout() -> impl IntoView {
    // Provided here, at the root, so every page below reads the same instance.
    provide_context(AuthStore::new());
    crate::v2::core::ui::toast::provide_toasts();
    // Restore a stored session on a cold load; a no-op for a guest with nothing saved.
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(crate::v2::core::api::client::bootstrap(expect_context::<
        AuthStore,
    >()));
    // The memo dedups by frame kind, so moving between two chromed routes never remounts the
    // sidebar or the top bar; only crossing a sign-in or editor boundary swaps the frame.
    let pathname = use_location().pathname;
    let frame_kind = Memo::new(move |_| classify_frame(&pathname.get()));
    let frame = move || match frame_kind.get() {
        // Bare: no chrome, and no wrapper element of any kind.
        FrameKind::Bare => view! { <AppRoutes /> }.into_any(),
        // Chromeless: the outlet owns the whole viewport.
        FrameKind::Chromeless => view! {
            <div class="h-screen w-screen overflow-hidden bg-background">
                <AppRoutes />
            </div>
        }
        .into_any(),
        // Chromed: sidebar, top bar, and the padded or full-bleed main region.
        FrameKind::Chrome => {
            let main_class = move || {
                if crate::router::full_bleed(&pathname.get()) {
                    "min-h-0 flex-1 bg-background overflow-hidden"
                } else {
                    "min-h-0 flex-1 bg-background overflow-y-auto p-6"
                }
            };
            // The narrow-viewport drawer renders no DOM at all while it is closed.
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
    // The toast viewport is a sibling of the frame, not a child, so a frame swap never unmounts
    // it. It renders no DOM while the queue is empty.
    view! {
        {frame}
        <crate::v2::core::ui::toast::ToastViewport />
    }
}

#[cfg(test)]
#[path = "tests/layout.rs"]
mod tests;
