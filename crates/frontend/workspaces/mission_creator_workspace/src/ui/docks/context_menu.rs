//! Context menu.
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

use crate::ui::docks::top_strip::{ARRANGE, ARRANGE_MIN_SELECTION, ArrangeKind};
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::host_state::entity_selection;

mod connection_types;
pub use connection_types::{ConnKind, FormationKind};
mod menu_entries;
pub use menu_entries::{ContextItem, MenuEntry};
mod menu_state;
#[cfg(test)]
pub use menu_state::MenuTake;
#[cfg(target_arch = "wasm32")]
pub use menu_state::MenuTarget;
pub use menu_state::{MenuState, resolve_target};
mod menu_geometry;
#[cfg(target_arch = "wasm32")]
use menu_geometry::MENU_BOUNDS;
#[cfg(test)]
pub(crate) use menu_geometry::selectable_indices;
pub use menu_geometry::step_highlight;
#[cfg(test)]
use menu_geometry::{menu_axis_position, menu_scroll_top};
#[cfg(target_arch = "wasm32")]
use menu_geometry::{place_context_menu, reveal_menu_row};
mod menu_dispatch;
#[cfg(target_arch = "wasm32")]
pub use menu_dispatch::{
    close, dispatch, register_canvas_context_menu, set_menu_signal, toggle_submenu,
};
pub mod menu_overlay;
#[cfg(target_arch = "wasm32")]
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
