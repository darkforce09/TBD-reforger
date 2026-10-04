//! The reading pane for one dispatch.
//!
//! **Role:** renders the selected dispatch's header chips, headline, byline, optional
//! thumbnail and body prose.
//! **Position:** the detail half of the board's split pane.
//! **Signals & state:** none — the dispatch is read from the stored payload the feed owns.
//! **Invariants:** a body is authored as plain text and is stored unsanitised, so every
//! paragraph is rendered as a text node and escaped exactly once. Rendering it as markup would
//! need a real sanitiser on the write path first, and escaping it a second time is what put
//! entity codes on screen. Optional fields are omitted by the backend when empty, so each is
//! gated on being non-empty rather than drawn as a blank slot, and a thumbnail address is
//! checked again here even though the writer already guarded it.

#[cfg(target_arch = "wasm32")]
use super::article_feed::{tag_label, tag_variant};
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::Announcement;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use frontend_ui::badge_class;
#[cfg(target_arch = "wasm32")]
use frontend_ui::datefmt::format_local_datetime;
#[cfg(any(target_arch = "wasm32", test))]
use http_url_guard::is_http_url;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

#[cfg(test)]
#[path = "tests/announcements.rs"]
mod tests;

/// A dispatch body split into its non-empty, blank-line separated paragraphs.
///
/// Pure, so the text contract can be pinned: bare `<` and `&` must survive into the strings
/// the view will escape once, never arriving pre-escaped.
#[cfg(any(target_arch = "wasm32", test))]
fn body_paragraph_texts(body: &str) -> Vec<String> {
    body.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

/// The body prose: one paragraph element per paragraph, each a text node.
///
/// Single newlines inside a paragraph are kept by the whitespace rule on the element, and
/// inline markup shows as written — this reader does not interpret markdown.
#[cfg(target_arch = "wasm32")]
fn body_paragraphs(body: &str) -> impl IntoView + use<> {
    body_paragraph_texts(body)
        .into_iter()
        .map(|p| {
            view! {
                <p class="whitespace-pre-line text-sm leading-relaxed text-on-surface-variant">
                    {p}
                </p>
            }
        })
        .collect_view()
}

/// The reading pane for one dispatch.
///
/// Reads `tag`, `is_pinned`, `title`, `published_at`, `author_id`, `thumbnail_url`,
/// `pushed_to_discord` and `body` from the dispatch.
#[cfg(target_arch = "wasm32")]
pub(super) fn reader(p: &Announcement) -> impl IntoView {
    let tag = p.tag.clone();
    let pinned = p.is_pinned;
    let title = if p.title.is_empty() {
        "Untitled Post".to_string()
    } else {
        p.title.clone()
    };
    let published = format_local_datetime(p.published_at.as_deref().unwrap_or_default());
    let author = p.author_id.clone();
    let thumb = p.thumbnail_url.clone();
    let pushed = p.pushed_to_discord;
    let body = p.body.clone();
    view! {
        <article class="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-10">
            <header class="flex flex-col gap-3 border-b border-outline-variant/30 pb-6">
                <div class="flex flex-wrap items-center gap-2">
                    <span class=badge_class(tag_variant(&tag))>{tag_label(&tag)}</span>
                    {pinned
                        .then(|| {
                            view! { <span class=badge_class("warning")>"Pinned"</span> }
                        })}
                    {pushed
                        .then(|| {
                            view! {
                                <span class="inline-flex items-center gap-1 font-mono text-xs text-on-surface-variant">
                                    <MaterialIcon name="forum" class="text-sm" />
                                    "Pushed to Discord"
                                </span>
                            }
                        })}
                </div>
                <h1 class="text-headline-md tracking-tight text-on-surface">{title}</h1>
                <div class="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-xs text-on-surface-variant">
                    <span class="inline-flex items-center gap-1">
                        <MaterialIcon name="account_circle" class="text-sm" />
                        {if author.as_str().is_empty() { "Command".to_string() } else { author.to_string() }}
                    </span>
                    <span>{published}</span>
                </div>
            </header>
            {thumbnail_img_src(&thumb)
                .map(|src| {
                    view! {
                        <img
                            src=src.to_string()
                            alt=""
                            class="max-h-72 w-full rounded-xl border border-white/10 object-cover"
                        />
                    }
                })}
            <div class="flex flex-col gap-4">{body_paragraphs(&body)}</div>
        </article>
    }
}

/// The thumbnail address to render, or `None` when it is not one this page will load.
#[cfg(any(target_arch = "wasm32", test))]
fn thumbnail_img_src(url: &str) -> Option<&str> {
    is_http_url(url).then_some(url)
}
