//! A manual's revision history: the paged list, the reads behind it, and an older revision on
//! view with its restore.
//!
//! **Role:** declares the history fetches, the revision panel and the revision view, and
//! re-exports what the article pane mounts.
//! **Position:** inside the article pane — the panel beside the article area, the view in it —
//! over `GET /wiki/{slug}/revisions` and `GET /wiki/{slug}/revisions/{revision}`.
//! **Signals & state:** none at this level; the article's state drives both resources.
//! **Invariants:** every signed-in member can read the history; only an administrator is offered
//! a restore, and a restore is a save like any other.

mod revision_fetches;
mod revision_list;
mod revision_view;

pub(super) use revision_fetches::{
    RevisionListResource, RevisionResource, revision_list_resource, revision_resource,
};
pub(super) use revision_list::revision_list;
pub(super) use revision_view::revision_view;
