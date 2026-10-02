//! Large native values expand in place with independent numbered continuations.
use super::super::{data::use_read, page::ViewerContext, source_inspector::display};
use crate::v2::core::api::dto::equipment_data_viewer::EquipmentSourcePage;
use leptos::prelude::*;
/// An in-place browser of a large field value, or of its expandable metadata with `metadata`:
/// lists the entries at the current JSON pointer page by page, opens nested arrays and objects,
/// steps back to the parent, and links entries that name a container.
#[component]
pub fn ValueDetails(
    node: String,
    property: String,
    #[prop(default = false)] metadata: bool,
) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let node = StoredValue::new(node);
    let property = StoredValue::new(property);
    let pointer = RwSignal::new(String::new());
    let cursor = RwSignal::new(String::new());
    let retry = RwSignal::new(0u64);
    let read = use_read::<EquipmentSourcePage>(Memo::new(move |_| {
        c.nav.get().inspection_request(
            "values",
            &c.generation.get(),
            &[
                ("node_id", &node.get_value()),
                ("property", &property.get_value()),
                ("pointer", &pointer.get()),
                ("cursor", &cursor.get()),
                ("kind", if metadata { "metadata" } else { "" }),
                ("retry", &retry.get().to_string()),
            ],
        )
    }));
    view! {<div class="dv-inline-value"><div class="dv-filters"><button on:click=move|_|{pointer.set(String::new());cursor.set(String::new());}>"Start"</button><button on:click=move|_|{let p=pointer.get_untracked();pointer.set(p.rsplit_once('/').map(|(p,_)|p).unwrap_or("").into());cursor.set(String::new());}>"↑ Parent"</button><code>{move||pointer.get()}</code></div>
        {move||read.value.get().map(|page|view!{<div>{page.items.into_iter().map(|item|{let raw=serde_json::from_str::<serde_json::Value>(&item.value_json).ok();let target=raw.as_ref().and_then(|v|v.get("node_id")).and_then(|v|v.as_str()).map(String::from).or_else(||if item.label=="node_id"{raw.and_then(|v|v.as_str().map(String::from))}else{None});let open=!item.expanded||matches!(item.value_kind.as_str(),"array"|"object")&&item.value_count>0;view!{<article><header><strong>{item.label}</strong>{target.map(|id|view!{<a href=c.nav.get_untracked().href(&[("node",&id),("property",""),("view","all"),("data_q","")])>"Open container →"</a>})}{open.then(||view!{<button on:click=move|_|{pointer.set(item.key.clone());cursor.set(String::new());}>"Open →"</button>})}</header>{if item.expanded{view!{<pre class="dv-value-full">{display(&item.value_json)}</pre>}.into_any()}else{view!{<small>{format!("{} · {} entries",item.value_kind,item.value_count)}</small>}.into_any()}}</article>}}).collect_view()}</div><div class="dv-value-pages"><small>{format!("{} entries · starting at {}",page.total,cursor.get().parse::<u64>().unwrap_or(0)+1)}</small><button on:click=move|_|cursor.set(String::new())>"First"</button>{page.next_cursor.map(|next|view!{<button on:click=move|_|cursor.set(next.clone())>"Next →"</button>})}</div>})}
        {move||read.error.get().map(|message|view!{<p>{message}</p><button on:click=move|_|retry.update(|n|*n+=1)>"Retry value"</button>})}
    </div>}
}
