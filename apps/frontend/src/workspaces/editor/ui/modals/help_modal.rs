//! Editor shortcuts and the floating Controls Hint.
//!
//! The editor binds twenty-six distinct `KeyboardEvent` codes across fifteen window-level keydown listeners in fourteen editor-surface modules, with forty-two bindings in total.
//! The catalog is checked against those live handlers so every binding has a help row.

#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::{cn, MaterialIcon};
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::session::layout::HOVER_FILL;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
use std::cell::Cell;

mod controls_hint;
mod shortcut_catalog;

#[cfg(target_arch = "wasm32")]
pub use controls_hint::ControlsHint;
#[cfg(test)]
use controls_hint::*;
#[cfg(target_arch = "wasm32")]
pub use controls_hint::{hint_shown, set_hint_shown};
#[cfg(test)]
pub use shortcut_catalog::Shortcut;
pub use shortcut_catalog::{GROUPS, SHORTCUTS};

/// The keymap census checks collisions and confirms that thirteen listeners claim Escape.
/// Each Escape claimant gates its response on the state of its own surface.
#[cfg(test)]
#[path = "tests/help_modal/keymap_census/mod.rs"]
pub(crate) mod keymap_census;

#[cfg(test)]
#[path = "tests/help_modal/shortcut_coverage.rs"]
mod help_modal_shortcut_coverage;

#[cfg(test)]
#[path = "tests/help_modal/controls_hint_close.rs"]
mod help_modal_controls_hint_close;

#[cfg(test)]
#[path = "tests/help_modal/arrange_shortcuts.rs"]
mod help_modal_arrange_shortcuts;

#[cfg(test)]
#[path = "tests/help_modal/source.rs"]
mod source;
