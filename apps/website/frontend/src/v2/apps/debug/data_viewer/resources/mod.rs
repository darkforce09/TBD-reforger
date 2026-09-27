//! The catalog remains mounted while the selected resource changes.
use super::{data::use_read, layout::*, page::ViewerContext};
use crate::v2::core::api::dto::equipment_data_viewer::EquipmentResourcePage;
use leptos::prelude::*;
mod resource_details;

#[component]
pub fn Resources() -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let list_url = Memo::new(move |_| c.nav.get().catalog_request(&c.generation.get(), false));
    let listing = use_read::<EquipmentResourcePage>(list_url);
    let selection = Memo::new(move |_| {
        let n = c.nav.get();
        let id = n.get("resource");
        if id.is_empty() {
            vec![]
        } else {
            vec![(c.generation.get(), id.to_owned())]
        }
    });
    view! {<div class="dv-workspace dv-catalog-workspace">
        <details class="dv-resource-chooser" open><summary>"Choose a resource"</summary>
        <Panel label="Resource list width"><div class="dv-panel-heading"><h3>{move||{let n=c.nav.get();if n.get("catalog_capability").is_empty(){"Resources".into()}else{title(n.get("catalog_capability"))}}}</h3><Search/></div>
            <div class="dv-filters">{[("","All"),("equipment","Equipment"),("vehicle","Vehicles"),("dependency","Dependencies")].into_iter().map(|(key,label)|view!{<a class:dv-active=move||c.nav.get().get("domain")==key href=move||c.nav.get().href(&[("domain",key),("resource_cursor","")])>{label}</a>}).collect_view()}</div>
            {move||(!c.nav.get().get("catalog_capability").is_empty()).then(||view!{<a class="dv-filter-clear" href=c.nav.get().href(&[("catalog_capability",""),("resource_cursor","")])>{format!("{} ×",title(c.nav.get().get("catalog_capability")))}</a>})}
            <Feedback error=listing.error loading=listing.loading/>
            {move||listing.value.get().map(|page|{let rows=page.items.into_iter().map(|r|LinkRow{href:c.nav.get_untracked().resource_link(&r.resource_id),selected:c.nav.get_untracked().get("resource")==r.resource_id,label:r.label,detail:r.resource_name,badge:r.domains.join(" · ")}).collect();view!{<VirtualList rows memory_key=list_url.get_untracked()/><Pager total=page.total next=page.next_cursor cursor_key="resource_cursor"/>}})}
        </Panel></details>
        <section class="dv-resource-main"><Show when=move||selection.get().is_empty()><div class="dv-empty"><h2>"Choose a resource"</h2><p>"Select an item to see its fields and values."</p></div></Show>
            <For each=move||selection.get() key=|key|key.clone() children=move |_|view!{<resource_details::ResourceDetails/>}/>
        </section>
    </div>}
}
