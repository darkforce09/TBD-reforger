//! Editor shortcuts and the floating Controls Hint.
//!
//! The editor binds twenty-six distinct `KeyboardEvent` codes across fifteen window-level keydown listeners in fourteen editor-surface modules, with forty-two bindings in total.
//! The catalog is checked against those live handlers so every binding has a help row.

#[cfg(target_arch = "wasm32")]
use frontend_ui::tokens::HOVER_FILL;
#[cfg(target_arch = "wasm32")]
use frontend_ui::{MaterialIcon, cn};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
use std::cell::Cell;

pub mod controls_hint;
mod shortcut_catalog;

#[cfg(target_arch = "wasm32")]
pub use controls_hint::ControlsHint;
#[cfg(target_arch = "wasm32")]
pub use controls_hint::{hint_shown, set_hint_shown};
#[cfg(test)]
pub use shortcut_catalog::Shortcut;
pub use shortcut_catalog::{GROUPS, SHORTCUTS};
