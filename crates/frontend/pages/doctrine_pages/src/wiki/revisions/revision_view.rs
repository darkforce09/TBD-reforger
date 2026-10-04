//! An older revision on view: its banner, its blocks, and for an administrator the restore
//! behind a confirmation.
//!
//! **Role:** renders the revision chosen in the panel — which revision it is, when and by whom
//! it was saved, its title, and its blocks — with a way back to the current
//! text and, for an administrator, "Restore this revision", which asks first and then saves the
//! revision's fields over the current revision.
//! **Position:** fills the article area while the article's `viewing` names a revision.
//! **Signals & state:** reads the revision resource, the page's `is_admin` and the article's
//! `busy`; writes the article's `viewing` and `restore_confirm_open`; the confirmation hands the
//! restore to the save submission.
//! **Invariants:** a fetched revision is shown only when it is the one on view, so a slow answer
//! for an earlier choice never stands in for the later one. The current revision offers no
//! restore; a restore names the article's current revision as its base.

use super::super::article::ArticleState;
use super::super::blocks::render_blocks;
use super::super::display_text::{load_failure_text, revision_byline};
use super::super::page_state::WikiPageState;
use super::super::saving::{SaveOrigin, restore_request, submit_save};
use super::revision_fetches::RevisionResource;
use frontend_api_dtos::wiki::WikiRevision;
use frontend_transport::client::Fetched;
use frontend_ui::Dialog;
use leptos::prelude::*;

/// The banner over a revision on view.
const BANNER_CLASS: &str = "mb-6 flex flex-wrap items-center gap-3 rounded-xl border border-primary/30 bg-primary/10 px-4 py-3";
/// A banner button.
const BANNER_BUTTON_CLASS: &str = "rounded-full border border-white/10 px-3 py-1 font-mono text-[11px] tracking-widest text-on-surface-variant uppercase hover:bg-white/5";
/// The restore button.
const RESTORE_BUTTON_CLASS: &str = "rounded-full border border-primary/40 bg-primary/15 px-3 py-1 font-mono text-[11px] tracking-widest text-primary uppercase hover:bg-primary/25 disabled:opacity-50";

/// The revision on view, over an article now at `current_revision`.
pub(in super::super) fn revision_view(
    page: WikiPageState,
    article_state: ArticleState,
    revision: RevisionResource,
    current_revision: i64,
) -> impl IntoView {
    move || {
        let wanted = article_state.viewing.get();
        match revision.get().flatten() {
            Some(Fetched::Data(loaded)) if Some(loaded.revision) == wanted => {
                loaded_revision(page, article_state, loaded, current_revision).into_any()
            }
            Some(Fetched::Failed(failure)) => view! {
                <div class=BANNER_CLASS>
                    <p class="text-sm text-error-alert">
                        {load_failure_text(&failure, "this revision")}
                    </p>
                    {back_button(article_state)}
                </div>
            }
            .into_any(),
            _ => view! { <p class="text-sm text-on-surface-variant">"Loading…"</p> }.into_any(),
        }
    }
}

/// The button that returns to the current text.
fn back_button(article_state: ArticleState) -> impl IntoView {
    view! {
        <button
            type="button"
            class=BANNER_BUTTON_CLASS
            on:click=move |_| article_state.viewing.set(None)
        >
            "Back to the current revision"
        </button>
    }
}

/// A fetched revision: its banner, its blocks and the restore confirmation.
fn loaded_revision(
    page: WikiPageState,
    article_state: ArticleState,
    loaded: WikiRevision,
    current_revision: i64,
) -> impl IntoView {
    let number = loaded.revision;
    let byline = revision_byline(
        &loaded.created_at,
        loaded.author_id.as_ref().map(|id| id.as_str()),
    );
    let title = loaded.title.clone();
    let blocks = render_blocks(&loaded.blocks);
    let restorable = number != current_revision;
    let revision = StoredValue::new(loaded);
    view! {
        <div role="status" class=BANNER_CLASS>
            <p class="min-w-0 flex-1 text-sm text-on-surface">
                <span class="font-mono font-bold">{format!("Revision {number}")}</span>
                " · "
                {byline}
                " · "
                {title}
            </p>
            {back_button(article_state)}
            {move || {
                (restorable && page.is_admin.get())
                    .then(|| {
                        view! {
                            <button
                                type="button"
                                class=RESTORE_BUTTON_CLASS
                                disabled=move || article_state.busy.get()
                                on:click=move |_| article_state.restore_confirm_open.set(true)
                            >
                                "Restore this revision"
                            </button>
                        }
                    })
            }}
        </div>
        {blocks}
        {restore_confirmation(page, article_state, revision, current_revision)}
    }
}

/// The confirmation in front of a restore of `revision` over `current_revision`.
fn restore_confirmation(
    page: WikiPageState,
    article_state: ArticleState,
    revision: StoredValue<WikiRevision>,
    current_revision: i64,
) -> impl IntoView {
    let number = revision.with_value(|revision| revision.revision);
    let on_confirm = move |_| {
        article_state.restore_confirm_open.set(false);
        let request = revision.with_value(|revision| restore_request(revision, current_revision));
        submit_save(page, article_state, SaveOrigin::Restore, request);
    };
    view! {
        <Dialog open=article_state.restore_confirm_open title="Restore this revision?">
            <p class="mb-5 text-sm text-on-surface-variant">
                {format!(
                    "Revision {number} becomes the manual again — its text, title, category, icon \
                     and order — saved as a new revision after revision {current_revision}. Every \
                     revision stays in the history.",
                )}
            </p>
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    on:click=move |_| article_state.restore_confirm_open.set(false)
                    class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                >
                    "Cancel"
                </button>
                <button
                    type="button"
                    on:click=on_confirm
                    prop:disabled=move || article_state.busy.get()
                    class="rounded-md bg-primary/20 px-3 py-1.5 text-label-md text-primary transition-colors hover:bg-primary/30 disabled:opacity-60"
                >
                    {format!("Restore revision {number}")}
                </button>
            </div>
        </Dialog>
    }
}
