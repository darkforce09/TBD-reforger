//! Top command strip menus, mission status, and editing controls.
#![allow(dead_code)]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

use crate::v2::apps::editor::shell::layout::{
    BTN_ICON, DISABLED_GLYPH, DIVIDER, HOVER_FILL, MENU_GUTTER, ROW_MENUS, ROW_TOOLS, STRIP_ROWS,
    TOGGLED_PLATE,
};
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::inspector::env::author_env;
use crate::v2::core::ui::{cn, MaterialIcon, Select, Slider};

/// Filled primary action styling for Save Version.
const ACTION_PRIMARY: &str = "shrink-0 rounded bg-primary px-2.5 py-0.5 text-xs font-medium text-on-primary transition-colors hover:bg-primary/90";

/// Outlined secondary action styling for the Export menu.
const ACTION_SECONDARY: &str = "shrink-0 rounded border border-outline-variant/40 px-2.5 py-0.5 text-xs font-medium text-on-surface-variant";

/// Compact validation status chip geometry; severity colours apply to its count.
const VALIDATION_CHIP: &str = "flex shrink-0 items-center gap-1 rounded px-2 py-0.5 text-on-surface-variant transition-colors";

/// Dropdown row geometry shared by command and export menus.
const MENU_ROW: &str = "flex w-full items-center gap-1.5 px-3 py-1.5 text-left text-label-sm text-on-surface disabled:cursor-default disabled:text-outline";

/// flex box pushes it to the trailing edge; dimmer and smaller than the label because it is a hint,
/// Right-aligned keyboard chord hint within a menu row.
const MENU_CHORD: &str = "ml-auto pl-4 text-label-sm text-outline";

const MENU_PANEL: &str =
    "glass animate-menu-in absolute top-full z-50 mt-1 rounded-lg py-1 shadow-lg";

mod arrange;
mod clock_and_draft;
mod dialog_focus;
mod menu_catalog;
mod mission_summary;
mod row_mirror;
mod view;

use arrange::*;
/// Public Arrange commands and descriptors shared with other editor surfaces.
pub use arrange::{
    arrange_chord_for_label, arrange_for_code, run_arrange, ArrangeEntry, ArrangeKind, ARRANGE,
    ARRANGE_MIN_SELECTION,
};
/// Route identifier predicate shared with the mission settings mirror.
pub(crate) use clock_and_draft::is_mission_row_id;
use clock_and_draft::*;
/// Clock and draft recency formatting used by the strip and callers.
pub use clock_and_draft::{
    format_draft_recency, hhmm_to_minutes, minutes_to_hhmm, normalize_clock,
};
use dialog_focus::*;
use menu_catalog::*;
use mission_summary::*;
/// Mission census and generated summary helpers.
pub use mission_summary::{census_from_rows, summary_line, SlotCensus};
/// Mission row mirror shared with the settings dialog.
pub(crate) use row_mirror::RowMirror;
use row_mirror::*;
/// Top command strip component.
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
