//! The two content gates: signed in, and sufficiently privileged.
//!
//! **Role:** wrap a subtree so it renders only for a viewer who may see it, and show the right
//! stand-in otherwise.
//! **Position:** wraps the body of a page that reads account-scoped data.
//! **Signals & state:** each gate derives its admission from the session store's `bootstrapping`,
//! token and profile signals through a `Memo`, and renders from that memo alone, so a gate flips
//! to its content the moment a session lands.
//! **Invariants:** a gate must show its waiting state while the session is still being restored.
//! Rendering the signed-out prompt during bootstrap would flash a sign-in call to action at
//! somebody who is in fact signed in. A gate re-runs its children only when its admission changes:
//! a profile poll or a token rotation writes the session signals without changing who may see the
//! page, and re-running the children would re-mount the page and discard its state — an open
//! sheet, typed input, a booted editor.

use leptos::prelude::*;

use crate::v2::core::auth::AuthStore;
use crate::v2::core::auth::{has_min_role_authed, Role};

/// Where a viewer stands at the sign-in gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionAdmission {
    /// The stored session is still being restored, so nothing is decided yet.
    Restoring,
    /// No signed-in session.
    SignedOut,
    /// A signed-in viewer.
    Admitted,
}

/// The sign-in gate's admission, as a memo that notifies only when the admission changes.
pub(crate) fn session_admission(auth: AuthStore) -> Memo<SessionAdmission> {
    Memo::new(move |_| {
        if auth.bootstrapping.get() {
            SessionAdmission::Restoring
        } else if auth.is_authenticated() {
            SessionAdmission::Admitted
        } else {
            SessionAdmission::SignedOut
        }
    })
}

/// The administrator gate's admission, as a memo that notifies only when the viewer's role
/// crosses the administrator tier.
pub(crate) fn admin_admission(auth: AuthStore) -> Memo<bool> {
    Memo::new(move |_| has_min_role_authed(auth.user.get().map(|u| u.role), Role::Admin))
}

/// Render `children` only to a signed-in viewer.
///
/// Shows a waiting line while the session is being restored, and a sign-in call to action to a guest.
/// Wraps the body of any page that reads account-scoped data.
#[component]
pub fn AuthGate(children: ChildrenFn) -> impl IntoView {
    let admission = session_admission(expect_context::<AuthStore>());
    move || match admission.get() {
        SessionAdmission::Restoring => view! {
            <div class="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
                "Loading session…"
            </div>
        }
        .into_any(),
        SessionAdmission::SignedOut => view! {
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
        .into_any(),
        SessionAdmission::Admitted => children().into_any(),
    }
}

/// Render `children` only to a viewer whose role meets the administrator tier.
///
/// Shows a one-line refusal in place of the content otherwise — a control the viewer may not use
/// is better absent than present and refused. Wraps administration page bodies.
#[component]
pub fn AdminGate(children: ChildrenFn) -> impl IntoView {
    let admitted = admin_admission(expect_context::<AuthStore>());
    view! {
        <AuthGate>
            {
                let children = children.clone();
                view! {
                    <Show when=move || admitted.get() fallback=admin_access_required>
                        {children()}
                    </Show>
                }
            }
        </AuthGate>
    }
}

/// What a signed-in viewer below the administrator tier sees in place of an administration page.
fn admin_access_required() -> impl IntoView {
    view! {
        <div class="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
            "Admin access required."
        </div>
    }
}

#[cfg(test)]
#[path = "tests/gates.rs"]
mod tests;
