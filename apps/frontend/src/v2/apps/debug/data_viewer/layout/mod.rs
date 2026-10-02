//! Viewer-owned panel layout and bounded lists.
mod panels;
mod virtual_list;
use super::page::ViewerContext;
use leptos::prelude::*;
pub use panels::*;
pub use virtual_list::*;

/// The footer of a cursor-paged list: the total entry count and the current start position, a
/// link to the first page and, when the server returned `next`, a link to the next page, both
/// through the `cursor_key` query parameter.
#[component]
pub fn Pager(
    total: u64,
    next: Option<String>,
    #[prop(default = "cursor")] cursor_key: &'static str,
) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let start = c.nav.get().get(cursor_key).parse::<u64>().unwrap_or(0);
    view! {<div class="dv-pager"><span>{format!("{} entries · starting at {}",number(total),number(start+u64::from(total>0)))}</span><a href=c.nav.get().href(&[(cursor_key,"")])>"First"</a>{next.map(|v|view!{<a href=c.nav.get().href(&[(cursor_key,&v)])>"Next →"</a>})}</div>}
}
/// A search form that writes the submitted text into the `key` query parameter and resets the
/// list cursors, so every search is part of the shareable viewer location.
#[component]
pub fn Search(
    #[prop(default = "q")] key: &'static str,
    #[prop(default = "Find by name, GUID, path, component or field")] placeholder: &'static str,
) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let text = RwSignal::new(c.nav.get_untracked().get(key).to_owned());
    let navigate = leptos_router::hooks::use_navigate();
    view! {<form class="dv-search" on:submit=move|e|{e.prevent_default();navigate(&c.nav.get_untracked().href(&[(key,&text.get_untracked()),("cursor",""),("resource_cursor","")]),Default::default());}>
        <input aria-label=placeholder placeholder=placeholder prop:value=move||text.get() on:input=move|e|text.set(event_target_value(&e))/><button type="submit">"Find"</button>
    </form>}
}
/// The status line of a read: the error message when there is one, otherwise `Loading…` while
/// the read is loading, otherwise nothing. It is a `status` live region, so changes are announced.
#[component]
pub fn Feedback(error: RwSignal<Option<String>>, loading: RwSignal<bool>) -> impl IntoView {
    view! {<div role="status" class="dv-feedback">{move||if let Some(error)=error.get(){error}else if loading.get(){"Loading…".into()}else{String::new()}}</div>}
}
/// `value` with comma thousands separators, for example `12345` as `12,345`.
pub fn number(value: u64) -> String {
    let text = value.to_string();
    let mut result = String::new();
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}
/// A snake_case identifier as capitalised words, for example `native_type` as `Native Type`.
pub fn title(value: &str) -> String {
    value
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut c = word.chars();
            c.next()
                .map(|first| first.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
