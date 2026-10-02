//! Resource identity and tabs do not replace the filtered catalog.
use super::*;
/// The selected resource: its identity and counts, a notice when it lies outside the catalog
/// filter, record and source downloads, and its data, relationships and names sections. When the
/// resource does not exist in the selected generation, an absence notice replaces them.
#[component]
pub fn ResourceDetails() -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let read = use_read::<EquipmentResourcePage>(Memo::new(move |_| {
        c.nav
            .get()
            .inspection_request("resources", &c.generation.get(), &[])
    }));
    let membership = use_read::<EquipmentResourcePage>(Memo::new(move |_| {
        c.nav.get().catalog_request(&c.generation.get(), true)
    }));
    let section = Memo::new(move |_| c.nav.get().section().to_owned());
    view! {<Feedback error=read.error loading=read.loading/>
        {move||read.value.get().map(|p|if let Some(r)=p.items.into_iter().next(){view!{
            <header class="dv-resource-heading"><div class="dv-eyebrow">{r.resource_id}</div><h2>{r.label}</h2><p class="dv-path">{r.resource_name}</p><small>{format!("{} containers · {} source facts",number(r.node_count),number(r.fact_count))}</small>
                <Show when=move||membership.value.get().is_some_and(|p|p.total==0)><p class="dv-outside-catalog">"This resource is outside the current list filter. Your filtered list is preserved."</p></Show>
            </header>
            <nav class="dv-tabs dv-subtabs">{[("data","Data"),("relationships","Relationships"),("names","Names")].into_iter().map(|(s,label)|view!{<a class:dv-active=move||section.get()==s href=move||c.nav.get().href(&[("section",s),("node",""),("parent",""),("property",""),("pointer",""),("cursor","")])>{label}</a>}).collect_view()}<span class="dv-spacer"/>
                <a href=format!("/api/v1{}",c.nav.get_untracked().inspection_request("download",&c.generation.get_untracked(),&[("document","record")])) download>"Record ↓"</a><a href=format!("/api/v1{}",c.nav.get_untracked().inspection_request("download",&c.generation.get_untracked(),&[("document","source")])) download>"Source ↓"</a>
            </nav>
            {move||match section.get().as_str(){"relationships"=>view!{<super::super::relationships::Relationships/>}.into_any(),"names"=>view!{<super::super::source_inspector::DocumentInspector fixed_document="record" fixed_pointer="/names"/>}.into_any(),_=>view!{<super::super::resource_data::ResourceData/>}.into_any()}}
        }.into_any()}else{view!{<div class="dv-empty"><h2>"Resource is absent from this generation"</h2><p>"Choose another resource or open a previous generation."</p></div>}.into_any()})}
    }
}
