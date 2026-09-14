//! The dispatch feed: the master list, and the split-pane shell it shares with the reader.
//!
//! **Role:** orders the dispatches, renders one row per dispatch, and arranges the master list
//! beside the reading pane over the board's backdrop.
//! **Position:** the whole body of the announcements route.
//! **Signals & state:** the fetched rows are parked in a stored value that both the list and
//! the reader read. Selection is a memo over the route parameter, and clicking a row navigates
//! rather than setting a signal.
//! **Invariants:** the address bar is the single source of truth for which dispatch is open, so
//! a bare `/announcements` opens nothing and a deep link opens exactly one. Pinned dispatches
//! sort first and the server's order is preserved within each group. Nothing here refetches, so
//! nothing here can go stale.
#![allow(dead_code)]

use super::article_viewer::reader;
use crate::v2::core::ui::split_pane::{ListDetailItem, SplitPane, SplitPaneEmpty};
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::datefmt::format_short_date;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde_json::Value;

/// The string at `k`, or an empty string when the key is absent or is not a string.
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// The boolean at `k`, or `false` when the key is absent or is not a boolean.
pub(super) fn vbool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

/// The badge variant a dispatch tag is drawn in.
///
/// A tag this table has not learned yet degrades to the neutral chip rather than vanishing.
pub(super) fn tag_variant(tag: &str) -> &'static str {
    match tag {
        "modpack_update" => "primary",
        "event" => "tertiary",
        "important" => "error",
        _ => "neutral",
    }
}

/// The badge text for a dispatch tag: underscores become spaces, and an absent tag reads
/// `NOTICE`.
pub(super) fn tag_label(tag: &str) -> String {
    if tag.is_empty() {
        return "NOTICE".into();
    }
    tag.replace('_', " ").to_uppercase()
}

/// A row's preview line: the snippet the backend wrote, else the body's opening paragraph.
///
/// No truncation happens here — the row clamps the text to two lines itself.
pub(super) fn preview_text(p: &Value) -> String {
    let s = vstr(p, "snippet");
    if !s.is_empty() {
        return s;
    }
    let body = vstr(p, "body");
    body.split("\n\n").next().unwrap_or_default().to_string()
}

/// The board for `posts`: the ordered master list beside the reading pane.
///
/// Reads `id`, `is_pinned`, `tag`, `title`, `published_at`, `snippet` and `body` from each row.
pub(super) fn board(posts: Vec<Value>) -> impl IntoView {
    // Pinned first, then the server's order, which a stable sort preserves.
    let mut posts = posts;
    posts.sort_by_key(|p| !vbool(p, "is_pinned"));
    let posts = StoredValue::new(posts);
    let params = use_params_map();
    let selected = Memo::new(move |_| {
        params
            .read()
            .get("id")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    });
    let navigate = use_navigate();

    let master = view! {
        {move || {
            // Read selection so list highlight tracks navigation between dispatches.
            let sel = selected.get();
            posts
                .with_value(|posts| {
                    if posts.is_empty() {
                        return view! {
                            <p class="px-1 py-4 text-label-md text-on-surface-variant">
                                "No announcements yet."
                            </p>
                        }
                            .into_any();
                    }
                    posts
                        .iter()
                        .map(|p| {
                            let id = vstr(p, "id");
                            let click_id = id.clone();
                            let navigate = navigate.clone();
                            let pinned = vbool(p, "is_pinned");
                            let tag = vstr(p, "tag");
                            let title = vstr(p, "title");
                            let title = if title.is_empty() {
                                "Untitled Post".to_string()
                            } else {
                                title
                            };
                            let date = format_short_date(&vstr(p, "published_at"));
                            let preview = preview_text(p);
                            let is_active = sel.as_deref() == Some(id.as_str());
                            view! {
                                <ListDetailItem
                                    active=is_active
                                    meta=view! { {date} }.into_any()
                                    dot_class=if pinned { "bg-tactical-yellow" } else { "" }
                                    title=view! { {title} }.into_any()
                                    trailing=view! {
                                        <span class=badge_class(tag_variant(&tag))>
                                            {tag_label(&tag)}
                                        </span>
                                    }
                                        .into_any()
                                    preview=view! { {preview} }.into_any()
                                    on_click=Callback::new(move |()| {
                                        navigate(
                                            &format!("/announcements/{click_id}"),
                                            Default::default(),
                                        );
                                    })
                                />
                            }
                        })
                        .collect_view()
                        .into_any()
                })
        }}
    }
    .into_any();

    let detail = view! {
        {move || {
            let Some(id) = selected.get() else {
                // Nothing opened yet — including the case where there is nothing to open.
                return view! {
                    <SplitPaneEmpty
                        icon=view! { <MaterialIcon name="campaign" class="text-4xl" /> }.into_any()
                        message="Select a broadcast to read."
                    />
                }
                    .into_any();
            };
            posts
                .with_value(|posts| {
                    match posts.iter().find(|p| vstr(p, "id") == id) {
                        Some(p) => reader(p).into_any(),
                        // Deep link to an id that is not in the current feed.
                        None => {
                            view! {
                                <SplitPaneEmpty
                                    icon=view! { <MaterialIcon name="campaign" class="text-4xl" /> }
                                        .into_any()
                                    message="That broadcast is no longer in the feed."
                                />
                            }
                                .into_any()
                        }
                    }
                })
        }}
    }
    .into_any();

    let master_header = view! {
        <>
            <h2 class="text-headline-sm tracking-wide text-on-surface uppercase">"Comms Link"</h2>
            <MaterialIcon name="filter_list" class="text-outline" />
        </>
    }
    .into_any();

    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                <SplitPane transparent=true master_header=master_header master=master detail=detail />
            </div>
        </div>
    }
}
