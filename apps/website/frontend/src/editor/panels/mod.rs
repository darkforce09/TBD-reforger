//! T-934.5 — Eden docked panels & tool drawers (the T-661 `eden_*` split,
//! renamed per audit table 3.5). All ungated: they hold no wasm-only types
//! (doc-driving on:click bodies are cfg-gated inside the closures), so the
//! native view shell compiles them too.

pub mod attributes_modal;
// T-664 — right-click context menu.
pub mod context_menu;
pub mod dock_left;
pub mod dock_right;
pub mod env;
// T-692 — the Help menu's Controls Hint.
pub mod help_modal;
// T-159.22 — the left dock's Editor Layers tree data model (plain LayerRow/SlotRow,
// wasm-free, native-tested).
pub mod outliner;
// eden_tree.rs renamed at the T-934.5 move: the virtualized outliner renderer.
pub mod outliner_tree;
pub mod settings_modal;
pub mod toolbelt;
pub mod top_strip;
// T-655 — mission validation panel.
pub mod validation_panel;
pub mod vehicles_panel;
pub mod zones_panel;
