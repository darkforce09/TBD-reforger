//! Values are visible immediately; evidence expands without changing panels.
use super::super::{
    page::ViewerContext,
    source_inspector::{display, summary},
};
use super::{card_grid::GridContext, inline_details::InlineDetails};
use frontend_api_dtos::equipment_data_viewer::EquipmentSourcePageItemsItem;
use leptos::prelude::*;
/// The element id of the row for `property` on the card at `index`; the property name is
/// hex-encoded so any source name yields a valid, unique id.
pub(crate) fn field_id(index: usize, property: &str) -> String {
    format!(
        "dv-field-{index}-{}",
        property
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
/// One field of a container card: its name, value summary, native type and unit, toggling inline
/// details and a permalink on click. The expanded state lives in the grid's reading position, and
/// the row the location names scrolls itself into view.
#[component]
pub fn FieldRow(
    /// The field's source fact: its property, value and provenance.
    fact: EquipmentSourcePageItemsItem,
    /// The node id of the container the field belongs to.
    node: String,
    /// The field's position in its container.
    index: usize,
) -> impl IntoView {
    let grid = expect_context::<GridContext>();
    let c = expect_context::<ViewerContext>();
    let pair = StoredValue::new((node.clone(), fact.key.clone()));
    let stored = StoredValue::new(fact.clone());
    let expanded = Memo::new(move |_| grid.memory.with(|m| m.expanded.contains(&pair.get_value())));
    let value = if fact.expanded {
        let value = display(&fact.value_json);
        if value.is_empty() {
            "Empty string".into()
        } else {
            value
        }
    } else {
        summary(&fact)
    };
    let id = field_id(index, &fact.key);
    let target = StoredValue::new(id.clone());
    Effect::new(move |_| {
        let n = c.nav.get();
        let (source, property) = pair.get_value();
        if n.get("node") == source && n.get("property") == property {
            request_animation_frame(move || {
                if let Some(element) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id(&target.get_value()))
                {
                    element.scroll_into_view_with_bool(true);
                }
            });
        }
    });
    view! {<div class="dv-field" id=id data-property=fact.key.clone()>
        <button class="dv-field-toggle" aria-expanded=move||expanded.get() on:click=move|_|grid.memory.update(|m|{let key=pair.get_value();if !m.expanded.remove(&key){m.expanded.insert(key);}})>
            <span class="dv-field-name"><span>{move||if expanded.get(){"▾ "}else{"▸ "}}</span>{fact.key.clone()}</span><span class="dv-field-value" title=value.clone()>{value.clone()}</span>
            <small>{format!("{} · {}",fact.native_type.as_deref().unwrap_or(""),fact.native_unit.as_deref().unwrap_or("Unit unspecified"))}</small>
        </button>
        <Show when=move||expanded.get()><InlineDetails fact=stored.get_value() node=pair.get_value().0/>
            <a class="dv-field-permalink" href=move||{let(node,property)=pair.get_value();c.nav.get().href(&[("node",&node),("property",&property),("view",stored.get_value().view.as_deref().unwrap_or("all")),("section","data"),("data_q","")])}>"Link to this field →"</a>
        </Show>
    </div>}
}
