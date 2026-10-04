//! One scrolling data surface shows every native container without tree navigation.
pub mod card_grid;
pub mod component_card;
pub mod contents_navigation;
pub mod field_row;
mod grid_layout;
pub mod grid_loading;
pub mod inline_details;
mod observers;
pub mod value_details;
use super::{layout::Search, page::ViewerContext};
use leptos::prelude::*;

/// The data section of a resource: a configuration view selector, the contents menu and a field
/// search above one card grid. The grid remounts, with its own remembered reading position,
/// whenever the dataset, generation, resource, view or search changes.
#[component]
pub fn ResourceData() -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let navigate = leptos_router::hooks::use_navigate();
    let key = Memo::new(move |_| {
        let n = c.nav.get();
        format!(
            "{}|{}|{}|{}|{}",
            n.dataset(),
            c.generation.get(),
            n.get("resource"),
            n.get("view"),
            n.get("data_q")
        )
    });
    view! {<section class="dv-data-surface"><div class="dv-data-toolbar">
        <label>"Configuration "<select aria-label="Configuration view" prop:value=move||{let n=c.nav.get();if n.get("view").is_empty(){"effective".into()}else{n.get("view").to_owned()}} on:change=move|e|{navigate(&c.nav.get_untracked().href(&[("view",&event_target_value(&e)),("node",""),("property","")]),Default::default());}>
            <option value="effective">"Current"</option><option disabled=move||c.nav.get().dataset()=="gameplay" value="ancestor">"Ancestors"</option><option value="all">"All configurations"</option>
        </select></label><contents_navigation::ContentsNavigation/><Search key="data_q" placeholder="Find a component or field"/>
    </div><For each=move||vec![key.get()] key=|key|key.clone() children=move|key|view!{<card_grid::CardGrid memory_key=key/>}/></section>}
}
