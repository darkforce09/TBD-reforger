//! The state the wiki page shares across manuals: the session, the reading mode, the drafts.
//!
//! **Role:** bundles the page-level signals every pane reads into one `Copy` value, and defines
//! the read/edit mode and the unsaved draft of a manual.
//! **Position:** built once by the route component; handed to the index, the article pane, the
//! revision panel and the save submission.
//! **Signals & state:** `mode` (read or edit), `drafts` (one unsaved draft per slug), `search`
//! (the index filter), the `is_admin` memo, the page-list resource and the `AuthStore`.
//! **Invariants:** a draft records the revision its edit started from and keeps it for as long as
//! the draft lives, so a save always names the revision the author actually edited; only a
//! successful save, or a reload after a conflict, drops a draft.

#[cfg(target_arch = "wasm32")]
use crate::foundation::auth::AuthStore;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::client::Fetched;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::wiki::WikiPageSummary;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::DataEnvelope;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

/// Which half of the article surface the detail pane shows.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WikiMode {
    /// The rendered manual.
    Read,
    /// The raw markdown in a text area; administrators only.
    Edit,
}

/// An unsaved edit of one manual.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WikiDraft {
    /// The edited markdown.
    pub(super) body_md: String,
    /// The revision the edit started from: the `base_revision` its save sends.
    pub(super) base_revision: i64,
}

/// The draft after the author types `body_md` into a manual now at `article_revision`: a new
/// draft starts from that revision, an existing one keeps the revision it started from.
pub(super) fn updated_draft(
    existing: Option<&WikiDraft>,
    article_revision: i64,
    body_md: String,
) -> WikiDraft {
    WikiDraft {
        body_md,
        base_revision: existing.map_or(article_revision, |draft| draft.base_revision),
    }
}

/// The page list as the route fetched it.
#[cfg(target_arch = "wasm32")]
pub(super) type PageListResource = LocalResource<Fetched<DataEnvelope<WikiPageSummary>>>;

/// The signals the wiki page shares across its panes.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(super) struct WikiPageState {
    /// The session every request is made with.
    pub(super) store: AuthStore,
    /// Whether the signed-in viewer is an administrator, re-read as the session changes.
    pub(super) is_admin: Memo<bool>,
    /// The index filter.
    pub(super) search: RwSignal<String>,
    /// Reading or editing.
    pub(super) mode: RwSignal<WikiMode>,
    /// The unsaved draft of each manual, keyed by slug.
    pub(super) drafts: RwSignal<HashMap<String, WikiDraft>>,
    /// The page list, refetched after a save so the index shows new titles and categories.
    pub(super) page_list: PageListResource,
}

#[cfg(test)]
#[path = "tests/page_state.rs"]
mod tests;
