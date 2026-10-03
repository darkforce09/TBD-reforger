//! The names a reader of the editing commands imports with
//! `use mission_editing_commands::prelude::*;`.

pub use crate::document_text::export_text::{
    apply_row_metadata_to_export, compile_diagnostics_summary, compiled_export_text,
    export_gesture_is_duplicate, live_doc_title, row_meta_missing_message,
};
pub use crate::document_text::merge_report::{duplicate_slot_id_report, format_merge_report};
pub use crate::document_text::selection_digest::{
    SelectedEntity, classnames_text, format_grid_ref, grid_position_text,
    resolve_selected_entities, selection_summary_text,
};
pub use crate::error::{Error, Result};
pub use crate::hosted_commands::{
    commit_document_edit, copy_selection, delete_selection, document_entities, paste_at_cursor,
    selection_entities,
};
