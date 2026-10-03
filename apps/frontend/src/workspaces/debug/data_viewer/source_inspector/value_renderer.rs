//! Explicit paged expansion keeps arrays ordered and numeric source text intact.
use super::super::{data::use_read, layout::*, page::ViewerContext};
use crate::foundation::transport::dto::equipment_data_viewer::{
    EquipmentSourcePage, EquipmentSourcePageItemsItem,
};
use leptos::prelude::*;

/// The display text of the JSON-encoded value `raw`: a JSON string is unquoted and unescaped, and
/// any other value keeps its source text, so numbers keep their original digits.
pub fn display(raw: &str) -> String {
    if raw.starts_with('"') {
        serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.into())
    } else {
        raw.into()
    }
}
/// The one-line summary of a field value on a card: a value not sent in full shows its kind and
/// entry count, text over 160 characters is cut to its first 140, and an empty string reads as
/// `Empty string`.
pub fn summary(item: &EquipmentSourcePageItemsItem) -> String {
    if !item.expanded {
        return format!(
            "{} · {} entries · Open to inspect",
            item.value_kind, item.value_count
        );
    }
    let text = display(&item.value_json);
    if text.chars().count() > 160 {
        format!(
            "{}… · Open complete value",
            text.chars().take(140).collect::<String>()
        )
    } else if text.is_empty() {
        "Empty string".into()
    } else {
        text
    }
}

/// A whole source document with a link to download the original: `fixed_document` when given,
/// otherwise the document the location names (`generation` by default), browsed from
/// `fixed_pointer`.
#[component]
pub fn DocumentInspector(
    #[prop(default = "")] fixed_document: &'static str,
    #[prop(default = "")] fixed_pointer: &'static str,
) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let kind = if fixed_document.is_empty() {
        let s = c.nav.get().get("document").to_owned();
        if s.is_empty() {
            "generation".into()
        } else {
            s
        }
    } else {
        fixed_document.into()
    };
    view! {<section class="dv-document"><header><h2>{if kind=="record"&&fixed_pointer=="/names"{"Names and owning source components".to_owned()}else{format!("{} document",title(&kind))}}</h2><p class="dv-muted">"Open any object or numbered array entry. Original values and their order are preserved."</p><a href=format!("/api/v1{}",c.nav.get().request("download",&c.generation.get(),&[("document",&kind)])) download>"Download original document ↓"</a></header><ValueBrowser document=kind pointer=fixed_pointer.to_owned()/></section>}
}

/// A paged browser of the entries at a JSON pointer in `document`, or in the selected property's
/// value when `document` is empty, starting from `pointer`. The pointer and page live in the
/// location, so every position can be shared.
#[component]
pub(super) fn ValueBrowser(
    #[prop(into)] document: String,
    #[prop(into)] pointer: String,
    #[prop(default = false)] metadata: bool,
) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let doc = StoredValue::new(document);
    let initial = StoredValue::new(pointer);
    let current = Memo::new(move |_| {
        let n = c.nav.get();
        if n.get("pointer").is_empty() {
            initial.get_value()
        } else {
            n.get("pointer").into()
        }
    });
    let read = use_read::<EquipmentSourcePage>(Memo::new(move |_| {
        let n = c.nav.get();
        let doc = doc.get_value();
        n.request(
            if doc.is_empty() {
                "values"
            } else {
                "documents"
            },
            &c.generation.get(),
            &[
                ("document", &doc),
                ("kind", if metadata { "metadata" } else { "" }),
                ("pointer", &current.get()),
                ("cursor", n.get("value_cursor")),
            ],
        )
    }));
    view! {<div class="dv-value-browser"><div class="dv-filters"><a href=move||c.nav.get().href(&[("pointer",&initial.get_value()),("value_cursor","")])>"Start"</a><a href=move||{let p=current.get();let parent=p.rsplit_once('/').map(|(a,_)|a).unwrap_or("");c.nav.get().href(&[("pointer",parent),("value_cursor","")])}>"↑ Parent"</a><code>{move||current.get()}</code></div><Feedback error=read.error loading=read.loading/>
        {move||read.value.get().map(|p|view!{<div class="dv-expanded-values">{p.items.into_iter().map(|item|{let open=!item.expanded||matches!(item.value_kind.as_str(),"array"|"object")&&item.value_count>0;view!{<article><header><strong>{item.label.clone()}</strong><span class="dv-badge">{format!("{} · {}",item.value_kind,item.value_count)}</span>{open.then(||view!{<a href=c.nav.get().href(&[("pointer",&item.key),("value_cursor","")])>"Open →"</a>})}</header>{if item.expanded{view!{<pre class="dv-value-full">{display(&item.value_json)}</pre>}.into_any()}else{view!{<p>"Open this value to view every entry."</p>}.into_any()}}</article>}}).collect_view()}</div><Pager total=p.total next=p.next_cursor cursor_key="value_cursor"/>})}
    </div>}
}
