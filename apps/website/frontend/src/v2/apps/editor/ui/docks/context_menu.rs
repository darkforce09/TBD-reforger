//! Context menu.
#![allow(dead_code)]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::entity_selection;
use crate::v2::apps::editor::ui::docks::top_strip::{ArrangeKind, ARRANGE, ARRANGE_MIN_SELECTION};
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::outliner::outliner;

mod connection_types;
pub use connection_types::{ConnKind, FormationKind};
mod menu_entries;
pub use menu_entries::{ContextItem, MenuEntry};
mod menu_state;
pub use menu_state::{resolve_target, MenuState, MenuTake, MenuTarget};
mod menu_geometry;
use menu_geometry::{menu_axis_position, menu_scroll_top, MENU_BOUNDS};
#[cfg(target_arch = "wasm32")]
use menu_geometry::{place_context_menu, reveal_menu_row};
pub use menu_geometry::{selectable_indices, step_highlight};
mod menu_dispatch;
#[cfg(target_arch = "wasm32")]
pub use menu_dispatch::{close, dispatch, open, set_menu_signal, toggle_submenu};
mod menu_overlay;
pub use menu_overlay::ContextMenuOverlay;

#[cfg(test)]
#[path = "tests/context_menu/menu_model.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/context_menu/escape_stack.rs"]
mod t726_context_menu_esc_stack;

#[cfg(test)]
#[path = "tests/context_menu/disabled_row_reasons.rs"]
mod t807_disabled_rows_show_why;

#[cfg(test)]
#[path = "tests/context_menu/arrange_submenu.rs"]
mod t939_4_arrange_in_the_context_menu;

#[cfg(test)]
#[path = "tests/context_menu/menu_geometry.rs"]
mod t939_4_menu_geometry;

#[cfg(test)]
#[path = "tests/context_menu/source.rs"]
mod test_source;
