//! The editor's rendered surfaces — every pixel the operator sees around the map.
//!
//! **Role:** groups the workspace's Leptos views by the surface they draw: the chrome docked
//! around the viewport, and the layer outliner those docks host.
//! **Position:** a leaf of `v2::apps::editor`. It reads the document through the map engine and
//! the session signals under `shell` and `bridge`; nothing under `v2::pages` reaches into it.
//! **Signals & state:** none of its own. Each surface subscribes to the host signals it draws
//! and writes back through the map engine's hosted commands.
//! **Invariants:** a surface renders and dispatches; it never mutates the document directly. A
//! view that touches `web_sys` gates the touching body, not the whole component, so the native
//! test build still compiles the surface.

/// The chrome docked around the map viewport: the left and right docks, the top command strip,
/// the bottom toolbelt and the right-click context menu.
pub mod docks;
/// The Editor Layers outliner: the tree the docks render, the node model behind it, and the
/// pointer-drag latch that reparents rows.
pub mod outliner;
