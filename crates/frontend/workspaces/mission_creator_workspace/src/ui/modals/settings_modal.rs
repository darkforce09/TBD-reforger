//! Mission settings, editor preferences, and authored-settings dialogs.

use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::ui::docks::top_strip::RowMirror;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::audio_emitters::audio_emitters_panel;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::env::ENV_UNCARRIED_NOTE;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::env::{
    FLOW_DEFAULT_BRIEFING_S, FLOW_DEFAULT_SAFESTART_S, FLOW_DEFAULT_TIMELIMIT_S, JIP_OPTIONS,
    SETTINGS_UNREAD_NOTE, author_env, fmt_duration_secs, parse_flow_seconds, read_flow_jip,
    read_flow_seconds,
};
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::radio_panel::radio_panel;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::spawn_modules::spawn_modules_panel;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::tasks_panel::tasks_panel;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::weather_timeline::weather_timeline_panel;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::win_conditions_card::win_conditions_card;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;

pub mod all_settings_dialog;
mod environment_sections;
pub mod mission_dialog;
mod mission_row_mirror;
mod mission_row_model;
mod mission_row_sections;
pub mod preferences_dialog;
mod presentation_model;
mod settings_catalog;
mod settings_navigation;

#[cfg(test)]
pub use all_settings_dialog::inert_settings_row_reason;
#[cfg(target_arch = "wasm32")]
use all_settings_dialog::*;
#[cfg(target_arch = "wasm32")]
use environment_sections::*;
#[cfg(target_arch = "wasm32")]
pub use mission_dialog::MissionSettingsDialog;
#[cfg(any(test, target_arch = "wasm32"))]
use mission_row_mirror::*;
#[cfg(any(test, target_arch = "wasm32"))]
use mission_row_model::*;
#[cfg(target_arch = "wasm32")]
use mission_row_sections::*;
#[cfg(target_arch = "wasm32")]
use preferences_dialog::*;
#[cfg(any(test, target_arch = "wasm32"))]
use presentation_model::*;
#[cfg(test)]
use settings_catalog::*;
pub use settings_catalog::{
    DiffState, SettingDefault, SettingOwner, SettingRow, aggregate_settings, fmt_setting_default,
    fmt_setting_value,
};
#[cfg(any(test, target_arch = "wasm32"))]
use settings_navigation::*;
pub use settings_navigation::{
    ALL_SETTINGS_NOTE, NO_DEFAULT_DECLARED, NOT_A_SCHEMA_KEY, OWNER_UNRESOLVED_NOTE,
    owner_is_routable,
};

thread_local! {
    static PREFS_OPEN: std::cell::RefCell<Option<RwSignal<bool>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
fn set_prefs_signal(sig: RwSignal<bool>) {
    PREFS_OPEN.with(|p| *p.borrow_mut() = Some(sig));
}

/// Opens the user-local editor preferences dialog when it is mounted.
#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
fn set_all_settings_signal(sig: RwSignal<bool>) {
    ALL_SETTINGS_OPEN.with(|p| *p.borrow_mut() = Some(sig));
}

/// Opens the read-only settings list when it is mounted.
#[cfg(target_arch = "wasm32")]
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
