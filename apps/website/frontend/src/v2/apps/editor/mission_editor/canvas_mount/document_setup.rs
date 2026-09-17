//! Initializes the mission document and registers document commands.

use super::*;
use std::cell::Cell;
use std::rc::Rc;

/// Seeds and registers the mission document before asynchronous restoration.
pub(super) fn initialize(
    auth: crate::v2::core::auth::AuthStore,
    mission_id: String,
    current_semver: RwSignal<Option<String>>,
) -> (mission_doc::DocHandle, Rc<Cell<u32>>) {
    let doc = mission_doc::new_seeded_doc();
    editor_context::seed_new_mission_template(&doc);
    let doc_ver = Rc::new(Cell::new(1u32));
    mission_doc::register_mission_doc(doc.clone(), doc_ver.clone());

    crate::v2::apps::editor::shell::document_commands::set_ctx(
        doc.clone(),
        auth,
        mission_id.clone(),
        current_semver,
    );
    crate::v2::apps::editor::shell::document_commands::register_editor_commands(doc.clone());

    (doc, doc_ver)
}
