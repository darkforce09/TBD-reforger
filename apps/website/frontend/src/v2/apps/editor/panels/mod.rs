//! The editor's two full-screen dialogs — the surfaces that overlay the workspace rather than
//! frame the map or edit a selection. Both ungated: they hold no wasm-only types (doc-driving
//! on:click bodies are cfg-gated inside the closures), so the native view shell compiles them too.

// T-692 — the Help menu's Controls Hint.
pub mod help_modal;
pub mod settings_modal;
