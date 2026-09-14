//! The page shown when no route matches.
//!
//! **Role:** owns the router's fallback view.
//! **Position:** rendered into `<main>` inside the standard frame, so the navigation around it
//! stays usable.
//! **Signals & state:** none. The view is static.
//! **Invariants:** this is a client-side fallback only — the server answers every path, so an
//! unknown path still arrives as a successful document load and is resolved here.

use leptos::prelude::*;

/// The fallback rendered when no route matches.
///
/// Rendered inside the standard frame, as the router's fallback, so a visitor who mistypes a
/// path keeps the sidebar and the top bar and can navigate onwards.
#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center py-24 text-center">
            <span class="text-6xl font-bold text-primary">"404"</span>
            <h1 class="mt-4 text-2xl font-bold">"Sector Not Found"</h1>
            <p class="mt-2 text-on-surface-variant">
                "The requested route does not exist in this AO."
            </p>
            <a href="/" class="mt-6 text-primary hover:underline">"Return to Dashboard"</a>
        </div>
    }
}
