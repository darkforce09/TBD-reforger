//! Counts and capability entry points come from the selected generation.
mod selection_summary;
use super::{
    data::use_read,
    layout::{Feedback, number, title},
    page::ViewerContext,
};
use crate::v2::core::api::dto::equipment_data_viewer::EquipmentDatasetStatus;
use leptos::prelude::*;
pub use selection_summary::SelectionSummary;

#[component]
pub fn Overview() -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let read = use_read::<EquipmentDatasetStatus>(Memo::new(move |_| {
        c.nav.get().request("overview", &c.generation.get(), &[])
    }));
    view! {<section class="dv-overview"><div class="dv-eyebrow">{move||if c.nav.get().dataset()=="gameplay"{"SELECTED GAMEPLAY DATA"}else{"COMPLETE SOURCE DIAGNOSTICS"}}</div><h2>"Explore the export"</h2><p class="dv-muted">"Start with a resource, explore its systems, then follow each value to its original source."</p>
        <Feedback error=read.error loading=read.loading/>
        {move||read.value.get().and_then(|d|d.overview).map(|d|view!{
            <div class="dv-cards">{[("equipment","Equipment",d.equipment,"Wearables, weapons, ammunition and utilities"),("vehicle","Vehicles",d.vehicles,"Vehicles and their installed systems"),("dependency","Dependencies",d.dependencies,"Shared configurations and linked source resources")].into_iter().map(|(domain,label,count,description)|view!{
                <a class="dv-card" href=c.nav.get().href(&[("tab","resources"),("domain",domain),("resource",""),("catalog_capability",""),("q","")])><span>{label}</span><strong>{number(count)}<small>" resources →"</small></strong><p>{description}</p></a>
            }).collect_view()}</div>
            <div class="dv-overview-grid"><section><h3>"Available information"</h3><div class="dv-capabilities">{let mut caps=d.capabilities.into_iter().collect::<Vec<_>>();caps.sort();caps.into_iter().map(|(cap,count)|view!{<a href=c.nav.get().href(&[("tab","resources"),("catalog_capability",&cap),("domain",""),("resource","")])><span>"▸ "{title(&cap)}</span><small>{number(count)}</small></a>}).collect_view()}</div>
                <a class="dv-wide-link" href=c.nav.get().href(&[("tab","fields"),("catalog_capability","")])>"Browse retained native fields →"</a></section>
            <aside class="dv-summary"><div class="dv-eyebrow">"PRESERVED SOURCE DATA"</div><strong>{format!("{:.2} GB",d.bytes as f64/1_000_000_000.)}</strong><dl><dt>"Resources"</dt><dd>{number(d.resources)}</dd><dt>"Containers"</dt><dd>{number(d.nodes)}</dd><dt>"Source facts"</dt><dd>{number(d.facts)}</dd><dt>"Native fields"</dt><dd>{number(d.fields)}</dd></dl><p>"Values stay in the original export. Only the information you open is loaded."</p></aside></div>
        })}
        <div class="dv-quick-links">{(c.nav.get().dataset()=="gameplay").then(||view!{<a href=c.nav.get().href(&[("tab","selection"),("cursor","")])>"Selection policy: included and excluded fields →"</a>})}{[("generation","","Export details"),(if c.nav.get().dataset()=="gameplay"{"native_types"}else{"generation"},if c.nav.get().dataset()=="gameplay"{""}else{"/type_hierarchy"},"Native types"),("generation","/reader_verification","Extraction results")].into_iter().map(|(document,pointer,label)|view!{<a href=c.nav.get().href(&[("tab","document"),("document",document),("pointer",pointer),("cursor","")])>{label}" →"</a>}).collect_view()}</div>
    </section>}
}

#[component]
pub fn GenerationHistory() -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let read = use_read::<EquipmentDatasetStatus>(Memo::new(move |_| {
        c.nav.get().request(
            "status",
            &c.generation.get(),
            &[("cursor", c.nav.get().get("cursor"))],
        )
    }));
    view! {<section class="dv-overview"><h2>"Export history"</h2><p class="dv-muted">"Open a preserved generation, or follow the latest successful import."</p><Feedback error=read.error loading=read.loading/>
        <a class="dv-wide-link" href=move||c.nav.get().href(&[("generation","latest"),("tab","overview")])>"Follow latest export →"</a>
        {move||read.value.get().map(|s|view!{<div class="dv-history">{s.generations.into_iter().map(|g|view!{<a class="dv-wide-link" href=c.nav.get().href(&[("generation",&g),("tab","overview"),("cursor",""),("node",""),("property","")])>{g.clone()}" →"</a>}).collect_view()}</div>{s.next_cursor.map(|v|view!{<a href=c.nav.get().href(&[("cursor",&v)])>"More generations →"</a>})}})}
    </section>}
}
