//! Mission settings, editor preferences, and authored-settings dialogs.

#![allow(dead_code)]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::docks::top_strip::RowMirror;
use crate::v2::apps::editor::ui::inspector::env::ENV_UNCARRIED_NOTE;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::inspector::env::{
    author_env, fmt_duration_secs, parse_flow_seconds, read_flow_jip, read_flow_seconds,
    FLOW_DEFAULT_BRIEFING_S, FLOW_DEFAULT_SAFESTART_S, FLOW_DEFAULT_TIMELIMIT_S, JIP_OPTIONS,
    SETTINGS_UNREAD_NOTE,
};
use crate::v2::apps::editor::ui::inspector::spawn_modules::spawn_modules_panel;
use crate::v2::apps::editor::ui::inspector::win_conditions_card::win_conditions_card;
use crate::v2::core::ui::MaterialIcon;

mod all_settings_dialog;
mod environment_sections;
mod mission_dialog;
mod mission_row_mirror;
mod mission_row_model;
mod mission_row_sections;
mod preferences_dialog;
mod presentation_model;
mod settings_catalog;
mod settings_navigation;

pub use all_settings_dialog::inert_settings_row_reason;
use all_settings_dialog::*;
use environment_sections::*;
pub use mission_dialog::MissionSettingsDialog;
use mission_dialog::*;
use mission_row_mirror::*;
use mission_row_model::*;
use mission_row_sections::*;
use preferences_dialog::*;
use presentation_model::*;
use settings_catalog::*;
pub use settings_catalog::{
    aggregate_settings, fmt_setting_default, fmt_setting_value, DiffState, SettingDefault,
    SettingOwner, SettingRow,
};
use settings_navigation::*;
pub use settings_navigation::{
    owner_is_routable, ALL_SETTINGS_NOTE, NOT_A_SCHEMA_KEY, NO_DEFAULT_DECLARED,
    OWNER_UNRESOLVED_NOTE,
};

thread_local! {
    static PREFS_OPEN: std::cell::RefCell<Option<RwSignal<bool>>> =
        const { std::cell::RefCell::new(None) };
}

fn set_prefs_signal(sig: RwSignal<bool>) {
    PREFS_OPEN.with(|p| *p.borrow_mut() = Some(sig));
}

/// Opens the user-local editor preferences dialog when it is mounted.
pub fn open_editor_preferences() {
    PREFS_OPEN.with(|p| {
        if let Some(sig) = *p.borrow() {
            sig.set(true);
        }
    });
}

thread_local! {
    static ALL_SETTINGS_OPEN: std::cell::RefCell<Option<RwSignal<bool>>> =
        const { std::cell::RefCell::new(None) };
}

fn set_all_settings_signal(sig: RwSignal<bool>) {
    ALL_SETTINGS_OPEN.with(|p| *p.borrow_mut() = Some(sig));
}

/// Opens the read-only settings list when it is mounted.
pub fn open_all_settings() {
    ALL_SETTINGS_OPEN.with(|p| {
        if let Some(sig) = *p.borrow() {
            sig.set(true);
        }
    });
}

#[cfg(test)]
#[path = "tests/aggregated_settings.rs"]
mod aggregated_settings;
#[cfg(test)]
#[path = "tests/briefing_mirror.rs"]
mod briefing_mirror;
#[cfg(test)]
#[path = "tests/dialog_escape_stack.rs"]
mod dialog_escape_stack;
#[cfg(test)]
#[path = "tests/editor_preferences.rs"]
mod editor_preferences;
#[cfg(test)]
#[path = "tests/mission_presentation.rs"]
mod mission_presentation;
#[cfg(test)]
#[path = "tests/mission_shape.rs"]
mod mission_shape;
#[cfg(test)]
#[path = "tests/mission_shape_flight.rs"]
mod mission_shape_flight;
#[cfg(test)]
#[path = "tests/player_count.rs"]
mod player_count;
#[cfg(test)]
#[path = "tests/settings_click_affordance.rs"]
mod settings_click_affordance;
#[cfg(test)]
#[path = "tests/settings_inert_row_accessibility.rs"]
mod settings_inert_row_accessibility;

#[cfg(test)]
#[path = "tests/source.rs"]
mod source;
