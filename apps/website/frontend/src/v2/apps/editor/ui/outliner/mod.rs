//! The Editor Layers outliner.
//!
//! **Role:** the hierarchical view of the document's editor layers and the slots filed under
//! them — the node model, the windowed renderer the docks draw with, and the drag latch that
//! refiles rows.
//! **Position:** a leaf of `v2::apps::editor::ui`, rendered inside the left dock and reused by
//! the right dock's palette and by the ORBAT manager dialog.
//! **Signals & state:** the drag latch is a thread-local in [`drag`]; the node rows themselves are
//! derived from the document on every rebuild and are never cached.
//! **Invariants:** a row action reaches the document through the map engine's layer operations,
//! and every drag arm builds a set the drop consumes, so a multi-row drag can never move its
//! anchor alone.

/// The pointer-drag latch and the drop planner: what a drag carries, and whether a given drop
/// target would accept it.
pub mod drag;
/// The node model: builds the folder-and-slot tree, the ORBAT rows and the active-layer
/// selection the tree renders.
pub mod outliner;
/// The shared dock-tree rendering: the depth guides, the windowed list and the one-row draw both
/// docks reuse.
pub mod tree;
