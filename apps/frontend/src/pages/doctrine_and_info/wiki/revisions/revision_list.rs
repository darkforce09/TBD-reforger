//! The revision panel: one page of the open manual's history, newest first, with a pager.
//!
//! **Role:** lists the revisions of the page of history on show — number, title and byline —
//! marks the current one and the one on view, and pages through older and newer entries.
//! Choosing a revision shows it in the article area; choosing the current one returns there.
//! **Position:** the aside beside the article area, below it on narrow screens.
//! **Signals & state:** reads the list resource and the article's `viewing` and
//! `revisions_page`; writes `viewing` and `revisions_page`.
//! **Invariants:** the pager reads the page numbers the server answered, not the ones asked for,
//! and never steps before page 1 or past the last page; its buttons and its "Page n of m" label
//! each stay on one line in the narrow history column.

use super::super::api_paths::revision_page_count;
use super::super::article::ArticleState;
use super::super::display_text::{load_failure_text, revision_byline};
use super::revision_fetches::RevisionListResource;
use crate::foundation::transport::client::Fetched;
use crate::foundation::transport::dto::wiki::{WikiRevisionPage, WikiRevisionSummary};
use leptos::prelude::*;

/// A revision row.
const ROW_CLASS: &str = "w-full rounded-lg border px-3 py-2 text-left transition-colors";
/// A row that is not on view.
const ROW_IDLE_CLASS: &str = "border-transparent hover:border-white/10 hover:bg-white/5";
/// The row on view.
const ROW_ACTIVE_CLASS: &str = "border-primary/40 bg-primary/10";
/// A pager button: fixed width, its label on one line.
const PAGER_BUTTON_CLASS: &str = "shrink-0 whitespace-nowrap rounded-full border border-white/10 px-3 py-1 font-mono text-[11px] tracking-widest text-on-surface-variant uppercase hover:bg-white/5 disabled:opacity-40";
/// The pager's "Page n of m" label: one line, taking the room the buttons leave and cut with an
/// ellipsis only when the column cannot hold it.
const PAGER_LABEL_CLASS: &str = "min-w-0 truncate text-center font-mono text-[11px] text-outline";

/// The revision panel of the open manual, whose current revision is `current_revision`.
pub(in super::super) fn revision_list(
    article_state: ArticleState,
    revisions: RevisionListResource,
    current_revision: i64,
) -> impl IntoView {
    view! {
        <section aria-label="Revision history" class="p-6">
            <p class="mb-3 font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
                "Revision history"
            </p>
            {move || match revisions.get() {
                None => {
                    view! { <p class="text-sm text-on-surface-variant">"Loading…"</p> }.into_any()
                }
                Some(Fetched::Failed(failure)) => {
                    view! {
                        <p class="text-sm text-error-alert">
                            {load_failure_text(&failure, "the revision history")}
                        </p>
                    }
                        .into_any()
                }
                Some(Fetched::Data(history)) => {
                    history_page(article_state, history, current_revision).into_any()
                }
            }}
        </section>
    }
}

/// One fetched page of the history and its pager.
fn history_page(
    article_state: ArticleState,
    history: WikiRevisionPage,
    current_revision: i64,
) -> impl IntoView {
    let WikiRevisionPage {
        items,
        page,
        per_page,
        total,
    } = history;
    if items.is_empty() {
        return view! { <p class="text-sm text-on-surface-variant">"No revisions yet."</p> }
            .into_any();
    }
    let page_count = revision_page_count(total, per_page);
    let rows = items
        .into_iter()
        .map(|item| revision_row(article_state, item, current_revision))
        .collect_view();
    view! {
        <ol class="space-y-1">{rows}</ol>
        <div class="mt-4 flex items-center justify-between gap-2">
            <button
                type="button"
                class=PAGER_BUTTON_CLASS
                disabled={page <= 1}
                on:click=move |_| article_state.revisions_page.set((page - 1).max(1))
            >
                "Newer"
            </button>
            <span class=PAGER_LABEL_CLASS>{format!("Page {page} of {page_count}")}</span>
            <button
                type="button"
                class=PAGER_BUTTON_CLASS
                disabled={page >= page_count}
                on:click=move |_| article_state.revisions_page.set((page + 1).min(page_count))
            >
                "Older"
            </button>
        </div>
    }
    .into_any()
}

/// One revision of the list; choosing it puts it on view, or returns to the current text when it
/// is the current revision.
fn revision_row(
    article_state: ArticleState,
    item: WikiRevisionSummary,
    current_revision: i64,
) -> impl IntoView {
    let revision = item.revision;
    let is_current = revision == current_revision;
    let byline = revision_byline(&item.created_at, item.author_id.as_deref());
    let on_view = move || {
        let viewing = article_state.viewing.get();
        viewing == Some(revision) || (viewing.is_none() && is_current)
    };
    view! {
        <li>
            <button
                type="button"
                aria-current=move || on_view().then_some("true")
                class=move || {
                    format!(
                        "{ROW_CLASS} {}",
                        if on_view() { ROW_ACTIVE_CLASS } else { ROW_IDLE_CLASS },
                    )
                }
                on:click=move |_| {
                    article_state.viewing.set((!is_current).then_some(revision));
                }
            >
                <span class="flex items-center gap-2 font-mono text-xs text-on-surface">
                    {format!("Revision {revision}")}
                    {is_current
                        .then(|| {
                            view! {
                                <span class="rounded border border-primary/40 px-1.5 text-[10px] tracking-widest text-primary uppercase">
                                    "current"
                                </span>
                            }
                        })}
                </span>
                <span class="mt-0.5 block truncate text-sm text-on-surface-variant">
                    {item.title}
                </span>
                <span class="block font-mono text-[11px] text-outline">{byline}</span>
            </button>
        </li>
    }
}
