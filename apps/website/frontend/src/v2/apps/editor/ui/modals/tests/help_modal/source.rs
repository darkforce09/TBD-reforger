//! Combined help implementation for source-inspection tests.

/// Returns all shipped help code before the external test declarations.
pub(super) fn production_source() -> String {
    let facade = include_str!("../../help_modal.rs")
        .split_once("#[cfg(test)]")
        .map_or(include_str!("../../help_modal.rs"), |(source, _)| source);
    [
        facade,
        include_str!("../../help_modal/shortcut_catalog.rs"),
        include_str!("../../help_modal/controls_hint.rs"),
    ]
    .join("\n")
}
