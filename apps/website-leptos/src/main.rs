//! T-159 — Leptos CSR entrypoint. Mounts the platform shell (T-159.2).
//!
//! The Aegis chrome (Sidebar shipped; TopNav next), router, auth, and the map/mission wasm hosting
//! land in later slices. Every slice is verified in a real headless browser via the gate harness
//! (S/V/R/T), not just `cargo check`.

mod announcements;
mod app_routes;
mod approvals;
mod audit;
mod auth;
mod client;
mod content;
mod dashboard;
mod datefmt;
mod deployments;
mod dto;
// T-159.17 warm editor session — sessionStorage marker; wasm32-only (uses web-sys/js-sys), gated
// like the doc host below.
#[cfg(target_arch = "wasm32")]
mod editor_session;
mod event_hub;
mod event_manager;
mod events;
mod layout;
mod leaderboards;
// T-159.16 MissionDoc host — all content is wasm32-only (links map-engine-core `doc`), so gate the
// module declaration like the engine block inside `mission_editor`.
#[cfg(target_arch = "wasm32")]
mod mission_doc;
mod mission_editor;
mod mission_overview;
mod missions;
mod modpacks;
mod mortar;
mod nav;
mod orbat_selection;
mod personnel;
mod router;
// T-159.18 Select / LMB pick foundation — links map-engine-core `camera`+`spatial` and web-sys, so
// wasm32-only, gated like the doc host + persist modules.
#[cfg(target_arch = "wasm32")]
mod select_tool;
mod server_control;
mod server_intel;
mod settings;
mod split_pane;
mod ui;
mod vehicles;
mod wiki;
// T-159.17 yrs IDB persist — IndexedDB (`idb` crate) + debounced writer; wasm32-only, gated like the
// doc host.
#[cfg(target_arch = "wasm32")]
mod yrs_persist;

// The wasm entry is a `#[wasm_bindgen(start)]`, not the bin `main`, because linking
// map-engine-render (T-159.15) pulls in ITS `#[wasm_bindgen(start)]` (the panic hook); wasm-bindgen
// runs every registered start, but a bare bin `main` is NOT one of them, so it would be skipped and
// the app would never mount. Declaring our mount as a start makes both run.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start_app() {
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

// The bin still needs a `main`; on wasm the start above drives the mount.
fn main() {}
