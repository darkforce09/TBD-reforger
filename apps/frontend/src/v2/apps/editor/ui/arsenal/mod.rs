//! The Arsenal's rendered surface — the panels the loadout editor draws inside its modal.
//!
//! **Role:** groups the Leptos views the Arsenal raises around a slot's loadout: the cargo
//! editor for a worn container, the paper-doll host that previews the result in 3D with an SVG
//! fallback, and the compatibility and attachment panels beside the pick rows.
//! **Position:** a leaf of `v2::apps::editor::ui`. The loadout domain it draws — the rows, the
//! compatibility graph, the serialization — lives under `v2::apps::editor::arsenal`, which mounts
//! these panels; nothing else calls them.
//! **Signals & state:** none of its own. Each panel takes the pick, cargo and catalog signals the
//! Arsenal owns and reports every edit back through the callback it was handed.
//! **Invariants:** a panel renders and reports; the document write belongs to the Arsenal's own
//! commit path, so nothing here reaches the document directly.

/// The Arsenal panels: the per-container cargo editor, the 3D doll host and its SVG paper-doll
/// fallback, and the compatibility and attachment readouts.
pub mod panels;
