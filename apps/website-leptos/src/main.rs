//! T-159.1 scaffold — Leptos CSR entrypoint.
//!
//! Renders a hello marker proving the wasm build + mount pipeline works end-to-end (verified in a
//! real headless browser via the gate harness, not just `cargo check`). The Aegis chrome, router,
//! auth, and the map/mission wasm hosting land in later slices (T-159.2+).

use leptos::prelude::*;

/// Root component. Inline styles so it renders before the Tailwind/Aegis CSS lands (T-159.2).
#[component]
fn App() -> impl IntoView {
    view! {
        <main
            data-t159-scaffold="1"
            style="min-height:100vh;display:flex;flex-direction:column;align-items:center;justify-content:center;\
                   background:#0d1322;color:#dde2f7;font-family:system-ui,sans-serif;gap:0.5rem"
        >
            <h1 style="font-size:30px;font-weight:700;letter-spacing:-0.02em">
                "TBD Reforger — Leptos"
            </h1>
            <p style="color:#c4c6d0;font-size:14px">"T-159.1 scaffold · CSR · wasm mount OK"</p>
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
