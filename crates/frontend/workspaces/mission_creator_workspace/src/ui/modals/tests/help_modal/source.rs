//! Combined help implementation for source-inspection tests.

use mission_creator_engine_bridge::test_support::production_half;

/// Returns all shipped help code before the external test declarations.
pub(super) fn production_source() -> String {
    let facade = production_half(include_str!("../../help_modal.rs"));
    [
        facade,
        include_str!("../../help_modal/shortcut_catalog.rs"),
        include_str!("../../help_modal/controls_hint.rs"),
    ]
    .join("\n")
}
