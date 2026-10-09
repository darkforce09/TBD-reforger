//! The Mission Creator's engine bridge and input layer: everything that drives the editor's
//! frames and turns the operator's pointer and keyboard into map commands.
//!
//! **Role:** owns the editor's side of the engine seam ([`bridge`]: the boot machine, the
//! frame-timing belt, the hosted mission document and its undo drive, the host state the hosted
//! commands read, the overlays laid over the map and the tactical-graphics belt) and the input
//! layer over it ([`input`]: the canvas gestures, the two window keydown dispatches and the
//! browser half of the interactive map tools). With `test_fixtures` it also exposes the
//! test-only hooks the upper editor crates' tests call (the boot percentage, the cache reset).
//! **Position:** the second Mission Creator crate: above `mission_creator_state`, the foundation
//! crates and the mission, editing, streaming and rendering crates; below the session, the
//! Arsenal and the workspace crates, which mount the canvas, register the callbacks the gestures
//! hand off to and drive the overlays.
//! **Signals & state:** the hosted document handle, the undo drive, the installed editor context,
//! the selection, the armed placement, the boot phase, the frame-timing samples and the gesture
//! closures are thread-local and tab-scoped; an authored change reaches the document only through
//! the hosted commands of `mission_editing_commands`.
//! **Invariants:** depends on no Mission Creator crate above `mission_creator_state`; a module
//! that touches `web_sys` or a live engine handle is `wasm32`-only, so the native build and tests
//! compile the pure half of the seam; the test-only hooks exist only in this crate's tests and with
//! the dev-only `test_fixtures` feature.

/// The editor's side of the engine seam: the boot machine, the frame-timing belt, the hosted
/// document and its undo drive, the host state, the overlays, the tactical-graphics belt and the
/// map-asset host.
pub mod bridge;
/// The input layer: the DOM pointer and keyboard events over the map, turned into map-engine
/// commands — the canvas gesture closures, the two window-level keydown dispatches, and the
/// browser half of the interactive map tools.
pub mod input;
/// The items most callers name, for `use mission_creator_engine_bridge::prelude::*;`.
pub mod prelude;
