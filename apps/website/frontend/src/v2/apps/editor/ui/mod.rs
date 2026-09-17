//! The editor's rendered surfaces — every pixel the operator sees around the map.
//!
//! **Role:** groups the workspace's Leptos views by the surface they draw: the chrome docked
//! around the viewport, the layer outliner those docks host, the inspectors that edit the
//! selected subject, and the full-screen dialogs raised over all of it.
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
/// The inspectors: the attribute modal, the zone, vehicle and environment panels, the mission
/// settings cards and the validation drawer — every surface that edits a selected subject.
pub mod inspector;
/// The full-screen dialogs over the workspace: the controls hint and shortcut reference, the
/// mission settings sheet, and the faction and ORBAT authoring dialogs.
pub mod modals;
/// The Editor Layers outliner: the tree the docks render, the node model behind it, and the
/// pointer-drag latch that reparents rows.
pub mod outliner;
