//! Top command strip menus, mission status, and editing controls.
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::env::author_env;
#[cfg(target_arch = "wasm32")]
use frontend_ui::tokens::DISABLED_GLYPH;
#[cfg(target_arch = "wasm32")]
use frontend_ui::tokens::HOVER_FILL;
#[cfg(target_arch = "wasm32")]
use frontend_ui::{MaterialIcon, Select, Slider, cn};
#[cfg(target_arch = "wasm32")]
use mission_creator_state::layout::{
    BTN_ICON, DIVIDER, MENU_GUTTER, ROW_MENUS, ROW_TOOLS, STRIP_ROWS, TOGGLED_PLATE,
};

/// Filled primary action styling for Save Version.
#[cfg(any(test, target_arch = "wasm32"))]
const ACTION_PRIMARY: &str = "shrink-0 rounded bg-primary px-2.5 py-0.5 text-xs font-medium text-on-primary transition-colors hover:bg-primary/90";

/// Outlined secondary action styling for the Export menu.
#[cfg(any(test, target_arch = "wasm32"))]
const ACTION_SECONDARY: &str = "shrink-0 rounded border border-outline-variant/40 px-2.5 py-0.5 text-xs font-medium text-on-surface-variant";

/// Compact validation status chip geometry; severity colours apply to its count.
#[cfg(target_arch = "wasm32")]
const VALIDATION_CHIP: &str = "flex shrink-0 items-center gap-1 rounded px-2 py-0.5 text-on-surface-variant transition-colors";

/// Dropdown row geometry shared by command and export menus.
#[cfg(target_arch = "wasm32")]
const MENU_ROW: &str = "flex w-full items-center gap-1.5 px-3 py-1.5 text-left text-label-sm text-on-surface disabled:cursor-default disabled:text-outline";

/// flex box pushes it to the trailing edge; dimmer and smaller than the label because it is a hint,
/// Right-aligned keyboard chord hint within a menu row.
#[cfg(target_arch = "wasm32")]
const MENU_CHORD: &str = "ml-auto pl-4 text-label-sm text-outline";

#[cfg(target_arch = "wasm32")]
const MENU_PANEL: &str =
    "glass animate-menu-in absolute top-full z-50 mt-1 rounded-lg py-1 shadow-lg";

mod arrange;
mod clock_and_draft;
mod dialog_focus;
mod menu_catalog;
mod mission_summary;
mod row_mirror;
pub mod view;

#[cfg(test)]
pub use arrange::arrange_for_code;
#[cfg(target_arch = "wasm32")]
pub use arrange::run_arrange;
#[cfg(any(test, target_arch = "wasm32"))]
use arrange::*;
/// Public Arrange commands and descriptors shared with other editor surfaces.
pub use arrange::{ARRANGE, ARRANGE_MIN_SELECTION, ArrangeKind, arrange_chord_for_label};
/// Route identifier predicate shared with the mission settings mirror.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use clock_and_draft::is_mission_row_id;
#[cfg(any(test, target_arch = "wasm32"))]
use clock_and_draft::*;
/// Clock and draft recency formatting used by the strip and callers.
pub use clock_and_draft::{
    format_draft_recency, hhmm_to_minutes, minutes_to_hhmm, normalize_clock,
};
#[cfg(target_arch = "wasm32")]
use dialog_focus::*;
#[cfg(any(test, target_arch = "wasm32"))]
use menu_catalog::*;
#[cfg(test)]
pub use mission_summary::SlotCensus;
#[cfg(test)]
use mission_summary::*;
/// Mission census and generated summary helpers.
pub use mission_summary::{census_from_rows, summary_line};
/// Mission row mirror shared with the settings dialog.
#[cfg(target_arch = "wasm32")]
pub(crate) use row_mirror::RowMirror;
#[cfg(any(test, target_arch = "wasm32"))]
use row_mirror::*;
/// Top command strip component.
#[cfg(target_arch = "wasm32")]
pub use view::TopCommandStrip;

#[cfg(test)]
#[path = "tests/top_strip/mission_summary_and_mirror.rs"]
mod mission_summary_and_mirror;

#[cfg(test)]
#[path = "tests/top_strip/menu_state_vocabulary.rs"]
mod menu_state_vocabulary;

#[cfg(test)]
#[path = "tests/top_strip/controls_hint_menu.rs"]
mod controls_hint_menu;

#[cfg(test)]
#[path = "tests/top_strip/form_controls.rs"]
mod form_controls;

#[cfg(test)]
#[path = "tests/top_strip/menu_row_layout.rs"]
mod menu_row_layout;

#[cfg(test)]
#[path = "tests/top_strip/escape_modal_stack.rs"]
mod escape_modal_stack;

#[cfg(test)]
#[path = "tests/top_strip/dialog_transient_exclusivity.rs"]
mod dialog_transient_exclusivity;

#[cfg(test)]
#[path = "tests/top_strip/save_version_dialog.rs"]
mod save_version_dialog;

#[cfg(test)]
#[path = "tests/top_strip/validation_chip.rs"]
mod validation_chip;

#[cfg(test)]
#[path = "tests/top_strip/arrange_actions.rs"]
mod arrange_actions;

#[cfg(test)]
#[path = "tests/top_strip/test_source.rs"]
mod test_source;
