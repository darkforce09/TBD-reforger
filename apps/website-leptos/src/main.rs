//! T-159 — Leptos CSR entrypoint. Mounts the platform shell (T-159.2).
//!
//! The Aegis chrome (Sidebar shipped; TopNav next), router, auth, and the map/mission wasm hosting
//! land in later slices. Every slice is verified in a real headless browser via the gate harness
//! (S/V/R/T), not just `cargo check`.

mod layout;
mod nav;
mod ui;

fn main() {
    // Guard the wasm-only mount so a native workspace-root `cargo build` still compiles this member
    // (trunk always builds wasm32, where this arm is live). The module bodies compile on both.
    #[cfg(target_arch = "wasm32")]
    {
        use layout::AppLayout;
        use leptos::prelude::*;
        console_error_panic_hook::set_once();
        leptos::mount::mount_to_body(|| view! { <AppLayout /> });
    }
}
