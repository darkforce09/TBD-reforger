//! T-159.1 scaffold — Leptos CSR entrypoint.
//!
//! Renders a hello marker proving the wasm build + mount pipeline works end-to-end (verified in a
//! real headless browser via the gate harness, not just `cargo check`). The Aegis chrome, router,
//! auth, and the map/mission wasm hosting land in later slices (T-159.2+).

use leptos::prelude::*;

/// Root component. Uses Aegis token utilities (`bg-background`, `text-headline-lg`, …) resolved by
/// the Tailwind v4 pipeline (T-159.2a) — the same tokens, byte-for-byte, the React app renders.
#[component]
fn App() -> impl IntoView {
    view! {
        <main
            data-t159-scaffold="1"
            class="flex min-h-screen flex-col items-center justify-center gap-2 bg-background text-on-surface"
        >
            <h1 class="text-headline-lg">"TBD Reforger — Leptos"</h1>
            <p class="text-label-md text-on-surface-variant">"T-159.2a · Aegis CSS pipeline"</p>
        </main>
    }
}

fn main() {
    // Guard the wasm-only mount so a native `cargo build` at the workspace root (which would pull in
    // this member) still compiles — trunk always builds wasm32, where this arm is live.
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        leptos::mount::mount_to_body(App);
    }
}
