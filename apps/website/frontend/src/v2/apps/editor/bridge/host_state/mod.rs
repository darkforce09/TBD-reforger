//! The host-side signal state the engine's hosted commands read.
//!
//! **Role:** holds what the browser knows and the engine must be told — the editor context
//! installed at load (the document, render engine and selection handles, and the signals that
//! mirror the document into the docks), the in-flight placement an operator picked up from a
//! palette, the selected-entity set, and the host half of undo grouping.
//! **Position:** inside the engine seam, beside the hosted document. A panel reads and writes this
//! state; `website_map_engine::editing` reads it back through the host closures it is handed.
//! **Signals & state:** thread-local, because the handles are `!Send` `Rc`s and cannot be passed
//! down a component tree. None of it is document state: an arm is never undoable, a selection
//! mints no undo step, and an unset signal is silence rather than an error.
//! **Invariants:** everything here reaches a live document or engine handle, so every module is
//! `#[cfg(target_arch = "wasm32")]` and its `pub mod` line carries the same gate. Each entry point
//! opens exactly one borrow of the context, and any document borrow is scoped to drop before the
//! post-edit tail opens its own read borrows.

/// The in-flight placement: what the operator picked up from a palette and has not yet dropped on
/// the map, and the release that commits it.
#[cfg(target_arch = "wasm32")]
pub mod armed_placement;
/// The editor context installed at load: the document, engine and selection handles every panel
/// reaches the open mission through, and the signals that mirror it into the docks.
#[cfg(target_arch = "wasm32")]
pub mod editor_context;
/// The selected entities: the id set, the renderer tint bound to it, the dock mirrors and the
/// camera move that frames it.
#[cfg(target_arch = "wasm32")]
pub mod entity_selection;
/// The gestures that must collapse into one undo step, and the confirmation a bulk gesture asks
/// before it commits.
#[cfg(target_arch = "wasm32")]
pub mod undo_grouped_gestures;
