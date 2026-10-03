//! The sign-in page.
//!
//! **Role:** owns the card a signed-out visitor lands on, and the button that hands the browser
//! to the backend's identity-provider flow.
//! **Position:** the `/login` route. The frame renders it bare — no sidebar, no top bar.
//! **Signals & state:** none. Nothing is read or written until the flow returns to the callback
//! route, which is where the session is established.
//! **Invariants:** the flow leaves the application, so it is started by setting the browser's
//! location rather than by a request. That only exists in the browser build, which is why the
//! window is looked up rather than assumed.

use leptos::prelude::*;

/// The sign-in card: the entry point of the identity-provider flow.
///
/// Renders the product name, one line of purpose, the button that starts the flow, and a link
/// out for a visitor who would rather browse signed out. The button navigates the whole page to
/// the backend's authorise endpoint rather than fetching it, because the flow continues with a
/// redirect off-site and returns to the callback route.
#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div class="flex min-h-screen flex-col items-center justify-center bg-background p-6">
            <div class="w-full max-w-md rounded-xl border border-border-subtle bg-surface-container p-8 text-center">
                <h1 class="text-2xl font-bold">
                    <span class="text-primary">"TBD"</span>
                    " Reforger"
                </h1>
                <p class="mt-2 text-on-surface-variant">
                    "Sign in to register, deploy, and manage operations."
                </p>
                <button
                    type="button"
                    class="mt-6 w-full rounded-lg bg-primary py-3 font-medium text-on-primary"
                    on:click=move |_| {
                        if let Some(win) = web_sys::window() {
                            let _ = win.location().set_href("/api/v1/auth/discord/login");
                        }
                    }
                >
                    "Sign in with Discord"
                </button>
                <a href="/" class="mt-4 block text-sm text-on-surface-variant hover:text-primary">
                    "Continue browsing without signing in"
                </a>
            </div>
        </div>
    }
}
