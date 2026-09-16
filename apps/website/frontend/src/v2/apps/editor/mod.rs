//! The scenario creator — the 2D/3D CAD workspace a mission is authored in.
//!
//! **Role:** owns every surface the operator drives while editing a scenario: the editor page
//! itself, the docked chrome around the map, the canvas mount and its overlays, the interactive
//! map tools, the arsenal loadout editor, and the reactive state that binds all of them to the
//! live document.
//! **Position:** a workspace under `v2::apps`, mounted full screen from the mission routes. It
//! consumes `v2::core` and the map and graphics engines; nothing under `v2::pages` reaches into
//! it, and no sibling workspace does either.
//! **Signals & state:** the document handle, the undo history, the selection, the armed placement
//! and the session signals live under [`state`]; every panel reads those signals and writes back
//! through the map engine's hosted commands.
//! **Invariants:** a document mutation travels through `website_map_engine::editing`, never
//! straight out of a panel. A module that touches `web_sys` or a live engine handle is
//! `#[cfg(target_arch = "wasm32")]`, and its `pub mod` line carries the same gate, so the native
//! test build still compiles the pure half of the workspace.

/// The loadout editor: the per-slot loadout rows and their compatibility rules, the asset
/// catalog behind the pickers, and the 3D paper doll that previews the result.
pub mod arsenal;
/// The map canvas: the boot machine, the viewport and frame-timing belt, the pointer and keyboard
/// gesture closures, the floating overlays, and the pure helper belt that feeds the renderer.
pub mod canvas;
/// The docked chrome's single import path — re-exports the panel components under [`panels`] so
/// consumers name one module rather than tracking which panel file holds which component.
pub mod eden_chrome;
/// The chrome inset constants and shared class recipes the strip, docks and toolbelt are laid out
/// from. [`tools::select_tool`] and [`mission_editor`] read the same constants back, so the pan,
/// select and marquee gates stay aligned with whatever the panels currently occupy.
pub mod layout;
/// The editor page itself: the route component that mounts the canvas, raises the chrome around
/// it, and wires the docks, tools and overlays to the document.
pub mod mission_editor;
/// The toolbelt's payload-size estimate for the mission as it would compile.
pub mod mission_size;
/// The docked panels and drawers: left and right docks, top strip, toolbelt, context menu, the
/// layer outliner, the attribute and environment inspectors, and the help and settings modals.
pub mod panels;
/// The reactive editor state: the document host and undo history, the selection, the armed
/// placement, persistence and hydration, tab locking, save status and session preferences, and
/// the command definitions the hotkeys dispatch.
pub mod state;
/// The browser half of the interactive map tools — ruler, line of sight, viewshed and select.
/// Each tool's state machine, geometry and verdicts live in `website_map_engine::editing::tools`;
/// what sits here is the DOM overlay and the pointer routing that drive them.
pub mod tools;
/// The map-asset host: fetches terrain and imagery bytes and hands them to the engine's streaming
/// host. Reaches `fetch` and a live engine handle, so it compiles on wasm only.
#[cfg(target_arch = "wasm32")]
pub mod world_assets;
/// Per-user world-layer visibility and basemap preferences, persisted to local storage. The wasm
/// host applies them to the chunk residency and the engine on each settle.
pub mod world_layer_prefs;
