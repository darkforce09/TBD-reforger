//! The editor's input layer: DOM pointer and keyboard events become map-engine commands.
//!
//! **Role:** owns every browser event the operator generates over the map and turns it into an
//! intent the rest of the editor can act on — the pointer, wheel, context-menu and double-click
//! gestures over the canvas, the two window-level keydown dispatches, and the browser half of the
//! interactive measure and selection tools.
//! **Position:** the entry edge of the workspace. It reads the live handles [`super::bridge`]
//! holds, writes through `website_map_engine::editing`, and flips the host signals under
//! [`super::bridge::host_state`]; nothing under [`super::ui`] routes events through here.
//! **Signals & state:** the gesture context bundles the handles and `Copy` signals every closure
//! captures, so one build hands the same environment to the pointer closures and the keydown
//! dispatch. Everything the closures own is tab-local; an authored change reaches the document
//! only through the engine's hosted commands.
//! **Invariants:** a module that touches `web_sys` is `#[cfg(target_arch = "wasm32")]` and its
//! `pub mod` line carries the same gate, so the native test build still compiles the pure half of
//! the layer. Undo and redo have exactly one entry point — `bridge::document_host::history` —
//! and the keydown
//! dispatch calls it across this boundary rather than stepping the document's stack itself.

/// The canvas gesture closures: wheel zoom, the pointerdown/move/up machine behind pan, marquee,
/// drag, rotate and the armed place, the context-menu finish and the double-click open, all
/// bundled behind the gesture context the window keydown shares.
#[cfg(target_arch = "wasm32")]
pub mod pointer_gestures;
/// The browser half of the interactive map tools — ruler, line of sight, viewshed and select.
/// Each tool's state machine, geometry and verdicts live in `website_map_engine::editing::tools`;
/// what sits here is the DOM overlay and the pointer routing that drive them.
pub mod tools;
/// The two window-level `keydown` dispatches: the editor's own chords (the shared Escape
/// dismissal, the clipboard, Select All, the dock latches, the snap grid and the widget variants)
/// and the undo/redo shortcuts that call into `document_host::history`.
#[cfg(target_arch = "wasm32")]
pub mod window_keydown;
