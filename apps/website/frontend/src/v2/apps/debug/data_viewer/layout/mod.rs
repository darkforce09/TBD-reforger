//! Viewer-owned panel layout and bounded lists.
mod panels;
mod virtual_list;
use super::page::ViewerContext;
use leptos::prelude::*;
pub use panels::*;
pub use virtual_list::*;

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
#[component]
pub fn Feedback(error: RwSignal<Option<String>>, loading: RwSignal<bool>) -> impl IntoView {
    view! {<div role="status" class="dv-feedback">{move||if let Some(error)=error.get(){error}else if loading.get(){"Loading…".into()}else{String::new()}}</div>}
}
pub fn number(value: u64) -> String {
    let text = value.to_string();
    let mut result = String::new();
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}
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
