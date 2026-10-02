//! One row of the post list: its date, its title and whether it is live.
//!
//! **Role:** the two state badges, and the row the master list renders per post.
//! **Position:** inside the master pane of the content route.
//! **Signals & state:** clicking a row writes `selected_id`; nothing else is held here.
//! **Invariants:** a post with no title yet still needs something to click, so it is listed under a
//! placeholder rather than as a blank row.
#![allow(dead_code)]

use super::doc::Doc;
use crate::v2::core::ui::split_pane::ListDetailItem;
use leptos::prelude::*;

/// Badge for a post that is live.
pub(super) const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";

/// Badge for a post that is still a draft.
pub(super) const BADGE_WARNING: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow";

/// One post in the master list, highlighted when it is the one being edited.
pub(super) fn article_row(
    d: Doc,
    active: bool,
    selected_id: RwSignal<Option<String>>,
) -> impl IntoView {
    let (badge, label) = if d.published {
        (BADGE_SUCCESS, "Published")
    } else {
        (BADGE_WARNING, "Draft")
    };
    let id_click = d.id.clone();
    let title = if d.title.is_empty() {
        "Untitled Post".to_string()
    } else {
        d.title.clone()
    };
    view! {
        <ListDetailItem
            active=active
            on_click=Callback::new(move |()| { selected_id.set(Some(id_click.clone())) })
            meta=view! { {d.date.clone()} }.into_any()
            title=view! { {title} }.into_any()
            trailing=view! { <span class=badge>{label}</span> }.into_any()
        />
    }
}
