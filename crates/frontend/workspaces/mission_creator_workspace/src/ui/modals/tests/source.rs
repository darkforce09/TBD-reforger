//! Test source assembled in production order for source-inspection assertions.

use mission_creator_engine_bridge::test_support::production_half;

/// Returns the complete mission-settings implementation without test declarations.
pub(super) fn production_source() -> String {
    let facade = production_half(include_str!("../settings_modal.rs"));
    [
        facade,
        include_str!("../settings_modal/mission_row_model.rs"),
        include_str!("../settings_modal/presentation_model.rs"),
        include_str!("../settings_modal/mission_row_mirror.rs"),
        include_str!("../settings_modal/mission_dialog.rs"),
        include_str!("../settings_modal/mission_row_sections.rs"),
        include_str!("../settings_modal/environment_sections.rs"),
        include_str!("../settings_modal/preferences_dialog.rs"),
        include_str!("../settings_modal/settings_catalog.rs"),
        include_str!("../settings_modal/settings_navigation.rs"),
        include_str!("../settings_modal/all_settings_dialog.rs"),
    ]
    .join("\n")
}
