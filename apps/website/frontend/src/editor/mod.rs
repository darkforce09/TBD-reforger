//! T-934.4 — Mission Creator & Eden 2D editor nest. Grows through the T-934
//! program: library/tools/world_assets first, panels/state/arsenal/canvas in
//! later batches.

// T-159.27/T-167 — Arsenal nest: loadout tab (arsenal/mod.rs), rules core, asset
// catalog, 3D doll.
pub mod arsenal;
// T-934.10 — canvas nest: the pure render-sync helper belt (`canvas/render_sync`), split out of
// `mission_editor`; overlays/boot/viewport/gestures follow in the later Phase B children.
pub mod canvas;
// T-159.21 Eden chrome — the docked shell was split by T-661 into the panel modules under
// `panels/`; `eden_chrome` stays the re-export shim so consumers' paths survive splits.
pub mod eden_chrome;
// T-661 — layout consts feed `tools/select_tool` / `mission_editor`.
pub mod layout;
pub mod library;
// The editor page itself — decomposes through Phase B/B2 of T-934.
pub mod mission_editor;
pub mod panels;
// T-934.6 — reactive state & document commands (editor_session / mission_* / yrs_persist /
// editor_ops before their renames).
pub mod state;
// T-172 B9 — pure SZ payload estimator (missionSize.ts port), native-tested.
pub mod mission_size;
pub mod tools;
// T-159.28 map-asset host (MVP: DEM hillshade) — fetch bytes + call the Rust dem core + engine
// tex_layer. wasm32-only (fetch + engine), gated like the doc host.
#[cfg(target_arch = "wasm32")]
pub mod world_assets;
// T-173 P6 — per-user world-layer visibility prefs + basemap view (localStorage). Pure/native-
// tested; the wasm host applies them to the residency + engine each settle.
pub mod world_layer_prefs;
