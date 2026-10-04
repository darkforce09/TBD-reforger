//! Native field combinations, observed metadata and exact occurrence links.
use super::{data::use_read, layout::*, page::ViewerContext};
use frontend_api_dtos::equipment_data_viewer::EquipmentFieldPage;
use leptos::prelude::*;
/// The Fields tab: every observed native field grouped by component class, with its effective,
/// ancestor and resource counts, units and system aliases, filterable by capability and
/// searchable. Selecting a field lists its exact occurrences, each linking to the source property
/// on the Resources tab.
#[component]
pub fn Fields() -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let read = use_read::<EquipmentFieldPage>(Memo::new(move |_| {
        let n = c.nav.get();
        n.request(
            "fields",
            &c.generation.get(),
            &[("cursor", n.get("cursor")), ("q", n.get("field_q"))],
        )
    }));
    view! {<section class="dv-fields"><header class="dv-panel-heading"><div class="dv-eyebrow">"EVERY OBSERVED NATIVE FIELD"</div><h2>"Field inventory"</h2><p class="dv-muted">"Grouped by native component class. Effective and ancestor occurrences stay separate; multiple system aliases never multiply a source fact."</p><Search key="field_q" placeholder="Find a component class or property"/>
        <div class="dv-filters"><a href=c.nav.get().href(&[("field",""),("class",""),("capability",""),("cursor","")])>"All fields"</a>
        {[("effective","Effective occurrences"),("ancestor","Ancestor occurrences"),("all","All occurrences")].into_iter().map(|(v,label)|view!{<a href=c.nav.get().href(&[("view",v),("cursor","")])>{label}</a>}).collect_view()}</div>
        <details><summary>"Browse by capability"</summary><div class="dv-filters">{move||c.status.get().and_then(|s|s.overview).map(|s|{let mut keys=s.capabilities.keys().cloned().collect::<Vec<_>>();keys.sort();keys.into_iter().map(|cap|view!{<a href=c.nav.get().href(&[("capability",&cap),("field",""),("cursor","")])>{title(&cap)}</a>}).collect_view()})}</div></details>
        </header><Feedback error=read.error loading=read.loading/>
        {move||read.value.get().map(|p|{
            let rows=if c.nav.get().get("field").is_empty(){p.items.into_iter().map(|f|LinkRow{href:c.nav.get().href(&[("field",&f.field_id.to_string()),("cursor","")]),label:format!("{} / {}",f.class_name,f.property),detail:format!("{} · {} effective · {} ancestor · {} resources · {}",f.native_type,number(f.effective_count),number(f.ancestor_count),number(f.resource_count),if f.units.is_empty(){"Unit unspecified".into()}else{f.units.join(", ")}),badge:if f.aliases.is_empty(){"Unmapped source field".into()}else{f.aliases.join(" · ")},selected:false}).collect()}else{p.occurrences.into_iter().map(|o|LinkRow{href:c.nav.get().href(&[("tab","resources"),("resource",o.resource_id.as_str()),("section","source"),("node",o.node_id.as_str()),("property",&o.property),("view",&o.view),("cursor",""),("pointer",""),("capability","")]),label:format!("{} / {}",o.label,o.property),detail:format!("{} · {}",o.resource_name,o.node_id),badge:format!("{} · {} · {}",o.view,o.origin,o.status),selected:false}).collect()};
            view!{<VirtualList rows/><Pager total=p.total next=p.next_cursor/>}
        })}
    </section>}
}
