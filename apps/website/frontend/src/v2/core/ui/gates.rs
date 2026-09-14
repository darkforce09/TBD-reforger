//! The two content gates: signed in, and sufficiently privileged.
//!
//! **Role:** wrap a subtree so it renders only for a viewer who may see it, and show the right
//! stand-in otherwise.
//! **Position:** wraps the body of a page that reads account-scoped data.
//! **Signals & state:** reads the session store's `bootstrapping` and profile signals reactively,
//! so a gate flips to its content the moment a session lands.
//! **Invariants:** a gate must show its waiting state while the session is still being restored.
//! Rendering the signed-out prompt during bootstrap would flash a sign-in call to action at
//! somebody who is in fact signed in.

use leptos::prelude::*;

use crate::shell::nav_config::{has_min_role_authed, Role};
use crate::v2::core::auth::AuthStore;

/// Render `children` only to a signed-in viewer.
///
/// Shows a waiting line while the session is being restored, and a sign-in call to action to a guest.
/// Wraps the body of any page that reads account-scoped data.
#[component]
pub fn AuthGate(children: ChildrenFn) -> impl IntoView {
    let auth = expect_context::<AuthStore>();
    move || {
        if auth.bootstrapping.get() {
            view! {
                <div class="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
                    "Loading session…"
                </div>
            }
            .into_any()
        } else if !auth.is_authenticated() {
            view! {
                <div class="flex min-h-[40vh] flex-col items-center justify-center gap-4 text-center">
                    <p class="text-on-surface-variant">
                        "Sign in to load live data from the platform."
                    </p>
                    <a
                        href="/login"
                        class="rounded-lg bg-primary px-6 py-2.5 text-sm font-medium text-on-primary"
                    >
                        "Sign in with Discord"
                    </a>
                </div>
            }
            .into_any()
        } else {
            children().into_any()
        }
    }
}

/// Render `children` only to a viewer whose role meets the administrator tier.
///
/// Shows nothing in place of the content otherwise — a control the viewer may not use is better absent
/// than present and refused. Wraps administration page bodies.
#[component]
pub fn AdminGate(children: ChildrenFn) -> impl IntoView {
    view! {
        <AuthGate>
            {
                let children = children.clone();
                move || {
                    let auth = expect_context::<AuthStore>();
                    if has_min_role_authed(auth.user.get().map(|u| u.role), Role::Admin) {
                        children().into_any()
                    } else {
                        view! {
                            <div class="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
                                "Admin access required."
                            </div>
                        }
                        .into_any()
                    }
                }
            }
        </AuthGate>
    }
}
