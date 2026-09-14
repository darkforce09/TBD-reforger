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
#![allow(dead_code)]

use super::article_feed::{tag_label, tag_variant, vbool, vstr};
use crate::v2::core::auth::url_guard;
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::datefmt::format_local_datetime;
use leptos::prelude::*;
use serde_json::Value;

#[cfg(test)]
#[path = "tests/announcements.rs"]
mod tests;

/// A dispatch body split into its non-empty, blank-line separated paragraphs.
///
/// Pure, so the text contract can be pinned: bare `<` and `&` must survive into the strings
/// the view will escape once, never arriving pre-escaped.
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
/// `pushed_to_discord` and `body` from the dispatch object.
pub(super) fn reader(p: &Value) -> impl IntoView + use<> {
    let tag = vstr(p, "tag");
    let pinned = vbool(p, "is_pinned");
    let title = vstr(p, "title");
    let title = if title.is_empty() {
        "Untitled Post".to_string()
    } else {
        title
    };
    let published = format_local_datetime(&vstr(p, "published_at"));
    let author = vstr(p, "author_id");
    let thumb = vstr(p, "thumbnail_url");
    let pushed = vbool(p, "pushed_to_discord");
    let body = vstr(p, "body");
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
                        {if author.is_empty() { "Command".to_string() } else { author }}
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
fn thumbnail_img_src(url: &str) -> Option<&str> {
    url_guard::is_http_url(url).then_some(url)
}
