//! Fixed-height navigation rows mount only the visible window plus overscan.
use leptos::prelude::*;
#[derive(Clone)]
pub struct LinkRow {
    pub href: String,
    pub label: String,
    pub detail: String,
    pub badge: String,
    pub selected: bool,
}
#[component]
pub fn VirtualList(
    rows: Vec<LinkRow>,
    #[prop(optional)] memory_key: Option<String>,
) -> impl IntoView {
    let context = expect_context::<super::super::page::ViewerContext>();
    let node = NodeRef::<leptos::html::Div>::new();
    let saved = memory_key
        .as_deref()
        .map(super::super::browsing_state::catalog_scroll)
        .unwrap_or(0);
    let key = StoredValue::new(memory_key);
    let offset = RwSignal::new((saved.max(0) as usize / 72).saturating_sub(3));
    Effect::new(move |_| {
        if let Some(n) = node.get() {
            n.set_scroll_top(saved);
        }
    });
    let length = rows.len();
    let rows = StoredValue::new(rows);
    let visible = ((web_sys::window()
        .and_then(|w| w.inner_height().ok())
        .and_then(|v| v.as_f64())
        .unwrap_or(1080.)
        / 72.)
        .ceil() as usize)
        + 8;
    view! {<div node_ref=node class="dv-virtual" tabindex="0" aria-label="Scrollable results"
        on:scroll=move|_|{if let Some(n)=node.get(){offset.set((n.scroll_top().max(0) as usize/72).saturating_sub(3));if let Some(k)=key.get_value(){super::super::browsing_state::save_catalog_scroll(k,n.scroll_top());}}}
        on:keydown=move|e|{let delta=match e.key().as_str(){"ArrowDown"=>72,"ArrowUp"=>-72,_=>0};if delta!=0{e.prevent_default();if let Some(n)=node.get(){n.set_scroll_top(n.scroll_top()+delta);}}}>
        <div style=move||format!("height:{}px;position:relative",length*72)>{move||rows.with_value(|rows|rows.iter().enumerate().skip(offset.get()).take(visible).map(|(i,row)|view!{
            <a class="dv-list-row" class:dv-selected={let href=row.href.clone();let selected=row.selected;move||if key.with_value(|k|k.is_some()){let n=super::super::navigation_state::Navigation::parse(href.split_once('?').map(|(_,q)|q).unwrap_or(""));n.get("resource")==context.nav.get().get("resource")}else{selected}} style=format!("position:absolute;top:{}px;height:72px;left:0;right:0",i*72) href=row.href.clone() title=format!("{}\n{}",row.label,row.detail)>
                <strong>{row.label.clone()}</strong><small>{row.detail.clone()}</small><span class="dv-badge">{row.badge.clone()}</span>
            </a>
        }).collect_view())}</div>
    </div>}
}
