//! Test source assembled in production order for source-inspection assertions.

/// Returns the complete mission-settings implementation without test declarations.
pub(super) fn production_source() -> String {
    let facade = include_str!("../settings_modal.rs")
        .split_once("#[cfg(test)]")
        .map_or(include_str!("../settings_modal.rs"), |(source, _)| source);
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
