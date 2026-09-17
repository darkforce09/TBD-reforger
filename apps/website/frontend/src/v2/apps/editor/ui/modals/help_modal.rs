//! Editor shortcuts and the floating Controls Hint.
//!
//! The editor binds twenty-six distinct `KeyboardEvent` codes across fifteen window-level keydown listeners in twelve editor-surface modules, with forty-two bindings in total.
//! The catalog is checked against those live handlers so every binding has a help row.

use crate::v2::apps::editor::shell::layout::HOVER_FILL;
use crate::v2::core::ui::{cn, MaterialIcon};
use leptos::prelude::*;
use std::cell::Cell;

mod controls_hint;
mod shortcut_catalog;

use controls_hint::*;
pub use controls_hint::{hint_shown, set_hint_shown, ControlsHint};
use shortcut_catalog::*;
pub use shortcut_catalog::{Shortcut, GROUPS, SHORTCUTS};

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
