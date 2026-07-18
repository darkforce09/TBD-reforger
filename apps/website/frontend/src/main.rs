//! T-159 — Leptos CSR entrypoint. Mounts the platform shell (T-159.2).
//!
//! The Aegis chrome (Sidebar shipped; TopNav next), router, auth, and the map/mission wasm hosting
//! land in later slices. Every slice is verified in a real headless browser via the gate harness
//! (S/V/R/T), not just `cargo check`.

mod announcements;
mod app_routes;
mod approvals;
// T-159.22 — flat registry rows → the Factions palette tree (the T-068.3 `buildCatalogTree` port).
// Pure data, no web-sys: ungated so its unit tests run on the native `cargo test` shell.
mod asset_catalog;
// T-159.26 — Attributes modal (AttributesModal.tsx port; wasm-gated internals).
mod attributes;
// T-159.27 — Arsenal loadout tab (ArsenalTab.tsx port; dumb Forge).
mod arsenal;
// T-167 — Smart-Arsenal domain core (arsenalRules.ts + arsenalDollModel.ts port; pure/native-tested).
mod arsenal_rules;
mod audit;
mod auth;
mod client;
mod content;
// T-159.25 — the Mission Library's transient "New Mission" dialog (CreateMissionDialog.tsx port).
mod create_mission_dialog;
mod dashboard;
mod datefmt;
mod deployments;
mod dto;
// T-159.22 dock commands — outliner select / active layer / palette drag-to-place. Drives the hosted
// MissionDocCore (add_slot / add_editor_layer), so wasm32-only, gated like the doc host.
#[cfg(target_arch = "wasm32")]
mod editor_ops;
// T-159.21 Eden chrome scaffold — the Mission Creator's docked shell (top strip / toolbelt / dock
// placeholders). Ungated: it holds no wasm-only types (the doc-driving on:click bodies are
// cfg-gated inside the closures), so the native view shell compiles it too.
mod eden_chrome;
// T-167 — Faction Manager dialog (FactionManagerDialog.tsx / T-153 port; /factions CRUD).
mod faction_manager;
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
// T-159.20 Save Version + Export — compile (map-engine-core `mission`) + authed POST + file download;
// wasm32-only, gated like the doc host.
#[cfg(target_arch = "wasm32")]
mod mission_commands;
mod mission_editor;
// T-159.26 server hydrate / conflict / dirty — GET /missions/:id → hydrate the saved version or
// prompt on a local-vs-server conflict. wasm32-only (auth GET + doc), gated like the doc host.
#[cfg(target_arch = "wasm32")]
mod mission_hydrate;
// T-159.28 map-asset host (MVP: DEM hillshade) — fetch bytes + call the Rust dem core + engine
// tex_layer. wasm32-only (fetch + engine), gated like the doc host.
#[cfg(target_arch = "wasm32")]
mod world_assets;
// T-159.21 undo/redo — drives the hosted MissionDocCore undo stack (+ the post-change glyph rebind
// and the `__editorHistory` bridge); wasm32-only, gated like the doc host.
#[cfg(target_arch = "wasm32")]
mod mission_history;
// T-172 B10 — the 3D arsenal doll mount (DollEngine, wasm-only like the map engine host).
#[cfg(target_arch = "wasm32")]
mod arsenal_doll;
mod mission_overview;
// T-172 B9 — pure SZ payload estimator (missionSize.ts port), native-tested.
mod mission_size;
mod missions;
mod modpacks;
mod mortar;
mod nav;
mod orbat_selection;
// T-159.22 — the left dock's Editor Layers tree (+ the "Unfiled" pseudo-root the seed forces). Owns
// plain LayerRow/SlotRow rather than `SlotSoa`, because map-engine-core is wasm32-only — so this
// stays ungated and its unit tests run on the native shell.
mod outliner;
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
// T-159.25 SSE consumer (useServerTelemetry port) — web-sys fetch/reader, wasm32-only.
#[cfg(target_arch = "wasm32")]
mod sse;
// T-159.25 — sonner replacement: Toasts context + top-right viewport (renders no DOM while empty).
mod toast;
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
