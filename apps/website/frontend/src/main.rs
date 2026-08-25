//! T-159 — Leptos CSR entrypoint. Mounts the platform shell (T-159.2).
//!
//! The Aegis chrome (Sidebar shipped; TopNav next), router, auth, and the map/mission wasm hosting
//! land in later slices. Every slice is verified in a real headless browser via the gate harness
//! (S/V/R/T), not just `cargo check`.

mod app_routes;
// T-934.1 — core framework utilities (auth, client, dto, sse, datefmt, toast, ui,
// url_guard, split_pane) and the app shell (layout, nav_config).
mod core;
mod shell;
// T-934.2/.3 — standard application pages (pages/{public,operations,admin}/…).
mod pages;
// T-934.4–.6 — the Mission Creator nest: library, tools, world assets, eden chrome
// panels, editor state, arsenal, and the editor page itself. Per-module provenance
// comments live in the folder mod.rs files.
mod editor;
mod router;

// The wasm entry is a `#[wasm_bindgen(start)]`, not the bin `main`, because linking
// map-engine-render (T-159.15) pulls in ITS `#[wasm_bindgen(start)]` (the panic hook); wasm-bindgen
// runs every registered start, but a bare bin `main` is NOT one of them, so it would be skipped and
// the app would never mount. Declaring our mount as a start makes both run.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start_app() {
    use leptos::prelude::*;
    use leptos_router::components::Router;
    use shell::layout::AppLayout;
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

// The bin still needs a `main`; on wasm the start above drives the mount.
fn main() {}
