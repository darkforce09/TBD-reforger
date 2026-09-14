//! The top bar: where the viewer is, and who the viewer is.
//!
//! **Role:** owns the header rendered above `<main>` in the chromed frame — the route breadcrumb,
//! the identity-link status pill, and the account menu with its sign-out action.
//! **Position:** the first child of the content column, beside the sidebar; the outlet sits
//! directly below it.
//! **Signals & state:** reads the session store from context and the live pathname; owns the
//! account menu's open/closed signal, which a click outside, a menu item, or the escape key all
//! close.
//! **Invariants:** the escape-key listener exists only in the browser build and is removed on
//! cleanup. Signing out revokes the refresh token before the local session is cleared, and the
//! cleared session is persisted immediately so a reload cannot restore it.

use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::{MaterialIcon, DEFAULT_AVATAR};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// The bar above `<main>`: a breadcrumb on the left, session state on the right.
///
/// Renders the route's breadcrumb, or the site name when the route declares none. On the right a
/// guest gets a single sign-in link; a signed-in viewer gets the identity-link pill, the avatar
/// button, and the menu it opens (settings, identity linking, sign out). The menu renders no DOM
/// while closed and is dismissed by a click anywhere or by the escape key.
#[component]
pub(crate) fn TopNav() -> impl IntoView {
    let pathname = use_location().pathname;
    let auth = expect_context::<AuthStore>();
    // Renders no DOM while closed.
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
    // Revoke the presented refresh token server-side without waiting on the reply, then clear
    // and persist the empty session so a reload cannot resurrect it.
    let sign_out = move |_| {
        menu_open.set(false);
        #[cfg(target_arch = "wasm32")]
        {
            let rt = auth.refresh_token.get_untracked();
            auth.clear_session();
            crate::v2::core::auth::persist(&auth.persist_state());
            leptos::task::spawn_local(async move {
                if let Some(rt) = rt {
                    let _ = crate::v2::core::api::client::api_post_ok(
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
            // A guest sees the sign-in link until the session lands; after that, the pill and
            // the avatar button that opens the account menu.
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
                        .map(|u| crate::v2::core::utils::safe_avatar_url(&u))
                        .unwrap_or_else(|| DEFAULT_AVATAR.to_string());
                    // Counts as linked only when the identity id is present and non-empty.
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
                                        <div class="glass animate-menu-in absolute top-full right-0 z-50 mt-2 w-56 rounded-lg py-1 shadow-lg">
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
