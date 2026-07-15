//! T-159 — Leptos CSR entrypoint. Mounts the platform shell (T-159.2).
//!
//! The Aegis chrome (Sidebar shipped; TopNav next), router, auth, and the map/mission wasm hosting
//! land in later slices. Every slice is verified in a real headless browser via the gate harness
//! (S/V/R/T), not just `cargo check`.

mod announcements;
mod app_routes;
mod auth;
mod client;
mod dashboard;
mod deployments;
mod dto;
mod layout;
mod nav;
mod router;
mod server_intel;
mod settings;
mod split_pane;
mod ui;

fn main() {
    // Guard the wasm-only mount so a native workspace-root `cargo build` still compiles this member
    // (trunk always builds wasm32, where this arm is live). The module bodies compile on both.
    #[cfg(target_arch = "wasm32")]
    {
        use layout::AppLayout;
        use leptos::prelude::*;
        use leptos_router::components::Router;
        console_error_panic_hook::set_once();
        // Mount inside a `<div id="root">` to mirror React's Vite mount node exactly (body > #root >
        // app). Beyond drop-in structural parity, it keeps the V-gate's positional-id numbering
        // aligned: dom.js numbers every [id] in document order, so a leading #root on ONE side would
        // offset every in-content id (e.g. #arma-link) on that side.
        leptos::mount::mount_to_body(|| {
            view! {
                <div id="root">
                    <Router>
                        <AppLayout />
                    </Router>
                </div>
            }
        });
    }
}
