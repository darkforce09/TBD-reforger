//! The head of the open manual: its stamps, category and title, and for an administrator the
//! read/edit switch and the save button.
//!
//! **Role:** renders the last-updated day, the current revision, the category and the title, and,
//! when the viewer is an administrator, the "[ READ ]"/"[ EDIT ]" switch and, while editing, the
//! "Save" button that sends the draft with the revision it started from.
//! **Position:** the top of the article pane, above the refusal notice and the body.
//! **Signals & state:** reads the page's `mode`, `drafts` and `is_admin`, and the article's
//! `busy`; the save button hands the draft save to the submission.
//! **Invariants:** the editing controls follow the reactive `is_admin` memo, so they never show to
//! a signed-out visitor or during the session restore. Saving with no draft re-sends the stored
//! body at the article's revision.

use super::super::display_text::calendar_day;
use super::super::page_state::{WikiMode, WikiPageState};
use super::super::saving::{draft_save_request, submit_save, SaveOrigin};
use super::ArticleState;
use crate::foundation::transport::dto::wiki::WikiArticle;
use leptos::prelude::*;

/// The neutral chip used for the stamps.
const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";
/// The save button.
const SAVE_BUTTON_CLASS: &str = "rounded-full border border-primary/40 bg-primary/15 px-4 py-1.5 font-mono text-xs tracking-widest text-primary uppercase hover:bg-primary/25 disabled:opacity-50";

/// The head of `article`.
pub(super) fn article_header(
    page: WikiPageState,
    article_state: ArticleState,
    article: StoredValue<WikiArticle>,
) -> impl IntoView {
    let (title, category, updated, revision) = article.with_value(|article| {
        (
            article.title.clone(),
            article.category.clone(),
            calendar_day(&article.updated_at),
            article.revision,
        )
    });
    view! {
        <header class="flex shrink-0 items-start justify-between gap-4 border-b border-white/10 px-8 pt-8 pb-5 md:px-12">
            <div class="min-w-0">
                <div class="mb-3 flex flex-wrap items-center gap-2">
                    <span class=BADGE_NEUTRAL>
                        <span class="material-symbols-outlined text-[14px]">"schedule"</span>
                        "Last updated "
                        {updated}
                    </span>
                    <span class=BADGE_NEUTRAL>
                        <span class="material-symbols-outlined text-[14px]">"history"</span>
                        "Revision "
                        {revision}
                    </span>
                    <span class="font-mono text-xs tracking-widest text-outline uppercase">
                        {category}
                    </span>
                </div>
                <h1 class="text-4xl font-bold tracking-tight text-white">{title}</h1>
            </div>
            {move || {
                page.is_admin
                    .get()
                    .then(|| {
                        view! {
                            <div class="flex shrink-0 flex-col items-end gap-2">
                                {read_edit_toggle(page.mode)}
                                {move || {
                                    (page.mode.get() == WikiMode::Edit)
                                        .then(|| save_button(page, article_state, article))
                                }}
                            </div>
                        }
                    })
            }}
        </header>
    }
}

/// The button that saves the open manual's draft.
fn save_button(
    page: WikiPageState,
    article_state: ArticleState,
    article: StoredValue<WikiArticle>,
) -> impl IntoView {
    let on_save = move |_| {
        let slug = article_state.slug.get_value();
        let draft = page
            .drafts
            .with_untracked(|drafts| drafts.get(&slug).cloned());
        let request = article.with_value(|article| draft_save_request(article, draft.as_ref()));
        submit_save(page, article_state, SaveOrigin::Draft, request);
    };
    view! {
        <button
            type="button"
            disabled=move || article_state.busy.get()
            class=SAVE_BUTTON_CLASS
            on:click=on_save
        >
            {move || if article_state.busy.get() { "Saving…" } else { "Save" }}
        </button>
    }
}

/// The read/edit switch shown to administrators.
fn read_edit_toggle(mode: RwSignal<WikiMode>) -> impl IntoView {
    let button = |target: WikiMode, label: &'static str| {
        view! {
            <button
                type="button"
                aria-pressed=move || (mode.get() == target).to_string()
                class=move || {
                    if mode.get() == target {
                        "rounded-full px-3 py-1 font-medium transition-all bg-surface-glass text-on-surface shadow-md"
                    } else {
                        "rounded-full px-3 py-1 font-medium transition-all text-on-surface-variant hover:text-on-surface"
                    }
                }
                on:click=move |_| mode.set(target)
            >
                {label}
            </button>
        }
    };
    view! {
        <div class="inline-flex shrink-0 gap-1 rounded-full border border-white/5 bg-black/20 p-1 font-mono text-xs">
            {button(WikiMode::Read, "[ READ ]")}
            {button(WikiMode::Edit, "[ EDIT ]")}
        </div>
    }
}
