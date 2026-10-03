//! The binary root of the single-page app: declares the five source layers and mounts the app.
//!
//! **Role:** the crate root. It declares the layers (`foundation`, `features`, `pages`,
//! `workspaces`, `shell`) and the render form of the route table (`app_routes`), and on wasm32 it
//! mounts the app frame into the document body from its start function.
//! **Position:** the top of the frontend's dependency order: foundation < features < pages,
//! workspaces < shell; the crate root and `app_routes` sit with the shell. Trunk builds this
//! binary for `apps/frontend/index.html`; every route is verified in a headless browser by the
//! gate harness as well as by `cargo check`.
//! **Signals & state:** none here; the session store and the toast queue are provided by the shell
//! layout this function mounts.
//! **Invariants:** a layer imports only layers below it; on a native build nothing mounts and
//! `main` is empty, so the mount chain (`app_routes`, the shell's frame components and every
//! route component) is compiled for wasm32 only, and `cargo test -p frontend` builds the native,
//! pure half of every layer.

// Wasm-only: its one user is the shell's `AppLayout`, which only `start_app` mounts.
/// The render form of the route table: each path bound to the component that renders it.
#[cfg(target_arch = "wasm32")]
mod app_routes;
/// Product capabilities shared by pages and workspaces, above the foundations.
mod features;
/// Shared foundations: the transport, the session, the route table, interface primitives.
mod foundation;
/// The routed document pages, one folder per navigation area.
mod pages;
/// The app frame around every route: layout, sidebar, top bar, not-found page.
mod shell;
/// The standalone full-screen workspaces: the Mission Creator and the debug benches.
mod workspaces;

#[cfg(test)]
#[path = "tests/doc_audit/mod.rs"]
mod doc_audit;

/// Mounts the app frame into the document body.
///
/// The wasm entry is this `#[wasm_bindgen(start)]` function, the only start function in the
/// linked module, so wasm-bindgen runs it when the module instantiates; the bin `main` stays empty.
/// It installs the panic hook before anything else runs, and no linked library installs one.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start_app() {
    use foundation::offline::offline_pack::OfflinePackRouteWatcher;
    use leptos::prelude::*;
    use leptos_router::components::Router;
    use shell::layout::AppLayout;
    console_error_panic_hook::set_once();
    // A sign-out deletes the departing account's local Mission Creator drafts, whether or not the
    // editor ever mounted in this tab; the session runs the hook without importing the editor.
    foundation::auth::logout_hooks::register_logout_hook(
        workspaces::editor::session::hydrate::purge_local_documents,
    );
    // The offline service worker registers before the app mounts; the watcher inside the router
    // downloads the offline pack on the first mortar calculator visit.
    foundation::offline::service_worker_registration::register_at_boot();
    // Mount inside a `<div id="root">` to mirror React's Vite mount node exactly (body > #root >
    // app). Beyond drop-in structural parity, it keeps the V-gate's positional-id numbering
    // aligned: dom.js numbers every [id] in document order, so a leading #root on ONE side would
    // offset every in-content id (e.g. #arma-link) on that side.
    leptos::mount::mount_to_body(|| {
        view! {
            <div id="root">
                <Router>
                    <AppLayout />
                    <OfflinePackRouteWatcher />
                </Router>
            </div>
        }
    });
}

// The bin still needs a `main`; on wasm the start above drives the mount.
fn main() {}
