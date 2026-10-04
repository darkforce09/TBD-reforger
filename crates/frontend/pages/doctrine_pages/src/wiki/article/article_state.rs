//! The state of one open manual: its reload counters, the revision on view, the write in flight
//! and its refusal.
//!
//! **Role:** bundles the signals the article pane, the revision panel and the save submission
//! share for the manual that is open, and the two reloads they trigger.
//! **Position:** created by the article pane each time a manual opens, so nothing carries over
//! from one manual to the next; handed by value (it is `Copy`) to every piece of that pane.
//! **Signals & state:** the slug; the article and revision-list reload counters; the page of the
//! history on show; the revision on view; whether a write is in flight; the last refusal; and
//! whether the restore confirmation is open.
//! **Invariants:** every reload goes through the counters, so the article and its history are
//! refetched by their own resources and never patched by hand. A reload after a write returns the
//! history to its first page, where the new revision is listed.

use super::super::page_state::WikiPageState;
use super::super::saving::{SaveFailure, SaveOrigin};
use leptos::prelude::*;

/// The signals of the open manual.
#[derive(Clone, Copy)]
pub(in super::super) struct ArticleState {
    /// The manual's slug.
    pub(in super::super) slug: StoredValue<String>,
    /// Bumped to refetch the article.
    pub(in super::super) article_generation: RwSignal<u32>,
    /// Bumped to refetch the page of the history on show.
    pub(in super::super) revisions_generation: RwSignal<u32>,
    /// The page of the history on show, from 1.
    pub(in super::super) revisions_page: RwSignal<i64>,
    /// The revision shown in place of the current one, if any.
    pub(in super::super) viewing: RwSignal<Option<i64>>,
    /// Whether a save or restore is in flight.
    pub(in super::super) busy: RwSignal<bool>,
    /// The last refused write, until the next write or reload.
    pub(in super::super) failure: RwSignal<Option<SaveFailure>>,
    /// Whether the restore confirmation is open.
    pub(in super::super) restore_confirm_open: RwSignal<bool>,
}

impl ArticleState {
    /// Fresh state for the manual `slug`.
    pub(in super::super) fn new(slug: String) -> Self {
        Self {
            slug: StoredValue::new(slug),
            article_generation: RwSignal::new(0),
            revisions_generation: RwSignal::new(0),
            revisions_page: RwSignal::new(1),
            viewing: RwSignal::new(None),
            busy: RwSignal::new(false),
            failure: RwSignal::new(None),
            restore_confirm_open: RwSignal::new(false),
        }
    }

    /// Refetch the article and the first page of its history, after a write landed.
    ///
    /// Fallible writes: the pane may have closed while the write was in flight.
    pub(in super::super) fn reload_after_write(&self) {
        self.revisions_page.try_set(1);
        self.article_generation
            .try_update(|generation| *generation += 1);
        self.revisions_generation
            .try_update(|generation| *generation += 1);
    }

    /// Fetch the manual as it is now, after a write was refused as a conflict.
    ///
    /// A refused draft save also drops the draft, so the editor opens on the latest text; a
    /// refused restore leaves any draft alone.
    pub(in super::super) fn reload_after_conflict(&self, page: WikiPageState, origin: SaveOrigin) {
        if origin == SaveOrigin::Draft {
            let slug = self.slug.get_value();
            page.drafts.update(|drafts| {
                drafts.remove(&slug);
            });
        }
        self.failure.set(None);
        self.restore_confirm_open.set(false);
        self.reload_after_write();
    }
}
