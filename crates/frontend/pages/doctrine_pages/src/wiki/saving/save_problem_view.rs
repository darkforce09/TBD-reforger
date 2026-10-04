//! The refusal notice of a wiki save or restore: what went wrong, the refused lines, and the
//! reload a conflict offers.
//!
//! **Role:** renders the article's recorded [`SaveFailure`](super::save_refusal::SaveFailure) as
//! an alert — the headline, one line per refused construct, and for a conflict the reload
//! button.
//! **Position:** shown under the article header, above the body, whenever a write was refused.
//! **Signals & state:** reads the article's `failure`; the reload button drops the draft (for a
//! refused draft save) and bumps the article's reload counters through
//! [`ArticleState::reload_after_conflict`].
//! **Invariants:** renders nothing while no write has been refused; every sentence is a text
//! node.

use super::super::article::ArticleState;
use super::super::page_state::WikiPageState;
use leptos::prelude::*;

/// The alert box.
const ALERT_CLASS: &str =
    "mx-8 mt-4 shrink-0 rounded-xl border border-error/40 bg-error/10 px-4 py-3 md:mx-12";
/// The reload button a conflict offers.
const RELOAD_BUTTON_CLASS: &str = "mt-3 rounded-full border border-primary/40 bg-primary/15 px-4 py-1.5 font-mono text-xs tracking-widest text-primary uppercase hover:bg-primary/25";

/// The notice of the article's refused write, or nothing.
pub(in super::super) fn save_problem_view(
    page: WikiPageState,
    article: ArticleState,
) -> impl IntoView {
    move || {
        article.failure.get().map(|failure| {
            let headline = failure.problem.headline();
            let lines = failure.problem.finding_lines();
            let reload = failure.reload_label();
            let origin = failure.origin;
            view! {
                <div role="alert" class=ALERT_CLASS>
                    <p class="text-sm text-error-alert">{headline}</p>
                    {(!lines.is_empty())
                        .then(|| {
                            view! {
                                <ul class="mt-2 space-y-1 font-mono text-xs text-on-surface-variant">
                                    {lines
                                        .into_iter()
                                        .map(|line| view! { <li>{line}</li> })
                                        .collect_view()}
                                </ul>
                            }
                        })}
                    {reload
                        .map(|label| {
                            view! {
                                <button
                                    type="button"
                                    class=RELOAD_BUTTON_CLASS
                                    on:click=move |_| article.reload_after_conflict(page, origin)
                                >
                                    {label}
                                </button>
                            }
                        })}
                </div>
            }
        })
    }
}
