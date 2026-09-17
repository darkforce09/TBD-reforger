//! The chrome docked around the map viewport.
//!
//! **Role:** the five surfaces that frame the canvas — the two side docks, the top command strip,
//! the bottom toolbelt and the floating right-click menu.
//! **Position:** a leaf of `v2::apps::editor::ui`, mounted by `mission_editor` and re-exported
//! under the chrome names in `shell::eden_chrome`.
//! **Signals & state:** none of its own; each dock subscribes to the host signals it draws.
//! **Invariants:** all five are ungated — they hold no wasm-only types, because the bodies that
//! drive the document are gated inside their event closures, so the native view shell compiles
//! them too.

/// The right-click menu over the viewport: the item model, the selection-aware hit resolution
/// that picks which take opens, and the floating overlay with its keyboard dismissal.
pub mod context_menu;
/// The left dock: the Editor Layers outliner tab and the named-locations index beside it.
pub mod dock_left;
/// The right dock: the Factions / Vehicles / Zones / Markers palette and the side chips that
/// drive the active side and the object place mode.
pub mod dock_right;
/// The bottom toolbelt: the floating mode toolbar (Select / Ruler / LoS) and the full-width
/// status bar carrying the cursor, object, selection and size read-outs.
pub mod toolbelt;
/// The top command strip: menu bar, editable title, time and weather scrubber, undo and redo,
/// save, export and settings, plus the mirror that debounces authored time and weather onto the
/// mission row.
pub mod top_strip;
