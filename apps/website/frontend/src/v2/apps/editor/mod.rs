//! The scenario creator — the 2D/3D CAD workspace a mission is authored in.
//!
//! **Role:** owns every surface the operator drives while editing a scenario: the editor page
//! itself, the docked chrome around the map, the canvas mount and its overlays, the interactive
//! map tools, the arsenal loadout editor, and the reactive state that binds all of them to the
//! live document.
//! **Position:** a workspace under `v2::apps`, mounted full screen from the mission routes. It
//! consumes `v2::core` and the map and graphics engines; nothing under `v2::pages` reaches into
//! it, and no sibling workspace does either.
//! **Signals & state:** the document handle, the undo history, the selection and the armed
//! placement live under [`bridge`]; the session signals live under [`shell`]; every panel reads
//! those signals and writes back through the map engine's hosted commands.
//! **Invariants:** a document mutation travels through `website_map_engine::editing`, never
//! straight out of a panel. A module that touches `web_sys` or a live engine handle is
//! `#[cfg(target_arch = "wasm32")]`, and its `pub mod` line carries the same gate, so the native
//! test build still compiles the pure half of the workspace.

/// The loadout editor: the per-slot loadout rows and their compatibility rules, the asset
/// catalog behind the pickers, and the 3D paper doll that previews the result.
pub mod arsenal;
/// The frontend's side of the engine seam: the canvas mount and its boot machine, the viewport and
/// frame-timing belt, the floating overlays, the tactical-graphics belt and the map-asset host.
pub mod bridge;
/// The input layer: the DOM pointer and keyboard events over the map, turned into map-engine
/// commands — the canvas gesture closures, the two window-level keydown dispatches, and the
/// browser half of the interactive map tools.
pub mod input;
/// The editor page itself: the route component that mounts the canvas, raises the chrome around
/// it, and wires the docks, tools and overlays to the document.
pub mod mission_editor;
/// The docked panels and drawers: left and right docks, top strip, toolbelt, context menu, the
/// layer outliner, the attribute and environment inspectors, and the help and settings modals.
pub mod panels;
/// The browser session the workspace runs inside: the draft writer and its status, the server
/// hydrate, cross-tab locking, the warm-session marker, the chrome and world-layer preferences,
/// the payload-size readout and the browser transport behind the document commands.
pub mod shell;
