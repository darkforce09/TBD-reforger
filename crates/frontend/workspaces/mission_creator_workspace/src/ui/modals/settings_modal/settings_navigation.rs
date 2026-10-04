//! Settings navigation for the mission settings interface.

use super::*;

/// Text displayed when the schema declares no default.
pub const NO_DEFAULT_DECLARED: &str = "no default declared";

/// Text displayed for editor-local keys outside the schema.
pub const NOT_A_SCHEMA_KEY: &str = "not a schema key (editor-local)";

/// Explains why an owner cannot be selected.
pub const OWNER_UNRESOLVED_NOTE: &str = "That owner could not be selected — it may have been \
                                         deleted since this list was built. Zones are selected in \
                                         the Zones panel of the right-hand dock.";

#[must_use]
/// Checks the registered selection route for a setting owner.
pub fn owner_is_routable(owner: &SettingOwner) -> bool {
    owner.subject_id().is_some_and(|id| {
        crate::ui::inspector::validation_panel::subject_id_routes(
            &mission_validation::SubjectId::new(id),
        )
    })
}

#[must_use]
/// Returns the row cursor style for its selection state.
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) fn row_cursor_class(clickable: bool) -> &'static str {
    if clickable {
        "cursor-pointer hover:bg-primary/10"
    } else {
        "cursor-default"
    }
}

/// Explains the read-only settings list and schema defaults.
pub const ALL_SETTINGS_NOTE: &str = "Every setting authored in this mission, whichever entity owns \
                                     it. Read-only: change a value where it is authored. Defaults \
                                     are read from mission.schema.json, so a key the schema states \
                                     no default for is shown as such rather than guessed.";

#[cfg(target_arch = "wasm32")]
#[must_use]
/// Reads the current document as JSON for settings aggregation.
pub(super) fn document_root() -> Option<serde_json::Value> {
    let handle = mission_creator_engine_bridge::bridge::document_host::history::doc_handle()?;
    let doc = handle.borrow();
    let core = doc.as_ref()?;
    serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok()
}
