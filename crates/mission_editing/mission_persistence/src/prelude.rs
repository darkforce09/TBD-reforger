//! The names a reader of the local draft decisions imports with
//! `use mission_persistence::prelude::*;`.

pub use crate::local_versus_server::{LocalDraftVerdict, classify_local_draft};
pub use crate::merge_policy::{apply_update_into_document, merge_before_write};
pub use crate::mission_id::is_uuid;
pub use crate::record_key::{
    ANONYMOUS_OWNER, owner_token_or_anonymous, scoped_key, snapshot_key, split_scoped_key,
};
pub use crate::record_read_retry::backoff_before_attempt_ms;
pub use crate::server_adoption::{Adopt, RowMeta, adopt_payload, apply_row_meta_only};
pub use crate::slot_fingerprint::slots_digest;
pub use crate::snapshot_slot::{SnapshotSlot, capture_document_snapshot};
pub use crate::stored_blob::restores_to_authored_content;
