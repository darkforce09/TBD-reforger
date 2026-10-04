//! Forward and reverse occurrences retain their native source context.
use super::{data::use_read, layout::*, page::ViewerContext};
use frontend_api_dtos::equipment_data_viewer::EquipmentRelationshipPage;
use leptos::prelude::*;
/// The relationships section of a resource: its outgoing references or incoming uses, filterable
/// by kind and configuration view. Selecting one shows its source component, property, method and
/// target, with links to the originating property and the referenced resource.
#[component]
pub fn Relationships() -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let read = use_read::<EquipmentRelationshipPage>(Memo::new(move |_| {
        let n = c.nav.get();
        n.request(
            "relationships",
            &c.generation.get(),
            &[("cursor", n.get("cursor")), ("node_id", "")],
        )
    }));
    view! {<section class="dv-relationships"><div class="dv-panel-heading"><h3>"Relationships"</h3><p class="dv-muted">"Every occurrence keeps its source component and property. External references name assets that are outside this export."</p>
        <div class="dv-filters">{[("outgoing","Referenced resources"),("incoming","Used by")].into_iter().map(|(v,label)|view!{<a href=c.nav.get().href(&[("direction",v),("cursor","")])>{label}</a>}).collect_view()}
        {[("","All kinds"),("gameplay","Gameplay"),("binary","External assets"),("native_type_match","Native type matches")].into_iter().map(|(v,label)|view!{<a href=c.nav.get().href(&[("kind",v),("cursor","")])>{label}</a>}).collect_view()}
        {[("effective","Effective"),("ancestor","Ancestors"),("all","All contexts")].into_iter().map(|(v,label)|view!{<a href=c.nav.get().href(&[("view",v),("cursor","")])>{label}</a>}).collect_view()}
        <a href=c.nav.get().href(&[("section","source"),("parent",""),("kind","all"),("capability","")])>"Contained components & parents →"</a></div></div>
        <Feedback error=read.error loading=read.loading/>
        {move||read.value.get().map(|p|{
            let selected=c.nav.get().get("relation").parse::<usize>().ok();
            let selected_row=selected.and_then(|i|p.items.get(i)).cloned();
            let rows=p.items.into_iter().enumerate().map(|(i,r)|{
                let incoming=c.nav.get().get("direction")=="incoming";
                LinkRow{href:c.nav.get().href(&[("relation",&i.to_string())]),label:if incoming{r.resource_name}else{r.target_resource_name},detail:format!("{} → {} · {}",r.node_id,r.property,r.method),badge:format!("{} · {} · Open relationship",r.kind,r.view),selected:selected==Some(i)}
            }).collect();
            view!{<VirtualList rows/><Pager total=p.total next=p.next_cursor/>
                {selected_row.map(|r|view!{<section class="dv-provenance"><div class="dv-eyebrow">"RELATIONSHIP DETAILS"</div><dl class="dv-metadata"><dt>"Resource"</dt><dd>{r.resource_name.clone()}</dd><dt>"Component"</dt><dd>{r.node_id.to_string()}</dd><dt>"Property"</dt><dd>{r.property.clone()}</dd><dt>"Context"</dt><dd>{r.view.clone()}</dd><dt>"Method"</dt><dd>{r.method.clone()}</dd><dt>"Target"</dt><dd>{r.target_resource_name.clone()}</dd></dl>
                    <div class="dv-native-links"><a href=c.nav.get().href(&[("resource",r.resource_id.as_str()),("section","source"),("node",r.node_id.as_str()),("property",&r.property),("view",&r.view),("cursor",""),("capability","")])>"Open originating source property →"</a>
                    {r.target_resource_id.map(|id|view!{<a href=c.nav.get().resource_link(&id)>"Open referenced resource →"</a>})}
                    {r.evidence_json.map(|e|view!{<div class="dv-provenance"><strong>"Type match · installation not verified"</strong>{super::source_inspector::metadata(&e)}</div>})}
                    {(r.kind=="binary").then(||view!{<p>"External binary asset; this bundle preserves its exact resource reference."</p>})}</div>
                </section>})}
            }
        })}
    </section>}
}
