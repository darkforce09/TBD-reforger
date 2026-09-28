//! A flat searchable contents menu jumps directly to an opaque container identity.
use super::super::{data::use_read, page::ViewerContext};
use crate::v2::core::api::dto::equipment_data_viewer::EquipmentResourceCardPage;
use leptos::prelude::*;
/// The "Jump to…" menu: searches the resource's containers by component, instance or field, pages
/// through the matches, and links straight to the chosen container. It only reads while open.
#[component]
pub fn ContentsNavigation() -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let open = RwSignal::new(false);
    let search = RwSignal::new(String::new());
    let query = RwSignal::new(String::new());
    let cursor = RwSignal::new(String::new());
    let read = use_read::<EquipmentResourceCardPage>(Memo::new(move |_| {
        if !open.get() {
            String::new()
        } else {
            c.nav.get().inspection_request(
                "resource-cards",
                &c.generation.get(),
                &[
                    ("kind", "contents"),
                    ("q", &query.get()),
                    ("cursor", &cursor.get()),
                ],
            )
        }
    }));
    // `aria-expanded` takes the literal tokens "true" and "false"; a `bool` value would render
    // a bare attribute when open and none when closed.
    let expanded = move || if open.get() { "true" } else { "false" };
    view! {<div class="dv-contents-control"><button aria-expanded=expanded on:click={move |_| open.update(|v| *v = !*v)}>"Jump to…"</button>
        <Show when=move||open.get()><div class="dv-contents-menu"><form on:submit=move|e|{e.prevent_default();query.set(search.get_untracked());cursor.set(String::new());}><input aria-label="Find a container" placeholder="Component, instance, or field" prop:value=move||search.get() on:input=move|e|search.set(event_target_value(&e))/><button>"Find"</button></form>
            {move||read.value.get().map(|page|view!{<div>{page.items.into_iter().map(|card|view!{<a href=c.nav.get_untracked().href(&[("node",&card.node_id),("property",""),("data_q","")]) on:click=move|_|open.set(false)><strong>{card.class_name}</strong><small>{card.node_id.clone()}</small></a>}).collect_view()}</div><div class="dv-contents-pages"><button on:click=move|_|cursor.set(String::new())>"First"</button>{page.next_cursor.map(|next|view!{<button on:click=move|_|cursor.set(next.clone())>"Next →"</button>})}<small>{format!("{} containers",page.total)}</small></div>})}
            {move||read.error.get()}
        </div></Show>
    </div>}
}
