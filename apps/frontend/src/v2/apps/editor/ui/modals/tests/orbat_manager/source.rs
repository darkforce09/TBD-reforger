//! Assembled ORBAT implementation for source-inspection tests.

/// Returns the dialog implementation without external test declarations.
pub(super) fn production_source() -> String {
    let facade = include_str!("../../orbat_manager.rs")
        .split_once("#[cfg(test)]")
        .map_or(include_str!("../../orbat_manager.rs"), |(source, _)| source);
    [
        facade,
        include_str!("../../orbat_manager/faction_templates.rs"),
        include_str!("../../orbat_manager/dialog_lifecycle.rs"),
        include_str!("../../orbat_manager/dialog.rs"),
        include_str!("../../orbat_manager/snapshot.rs"),
        include_str!("../../orbat_manager/tree_panel.rs"),
        include_str!("../../orbat_manager/tree_rows.rs"),
        include_str!("../../orbat_manager/slot_inspector.rs"),
        include_str!("../../orbat_manager/stats.rs"),
    ]
    .join("\n")
}
