//! The gameplay selection policy: the recorded decision for every reviewed native field.
//!
//! **Role:** renders the viewer's `selection` tab — a count per decision kind, the number of
//! diagnostic-only dependencies left out with a link to the publication receipt, and a
//! searchable, cursor-paged list of decisions, each with its native type, disposition, section
//! and reason.
//! **Position:** mounted by the viewer page when the location's tab is `selection`; reads the
//! `/debug/equipment-data/selection` endpoint through the viewer's cancellable read and takes the
//! location and generation from the shared viewer context.
//! **Signals & state:** one read owned by the component; the decision filter, search text and
//! page cursor live in the URL query (`selection_kind`, `selection_q`, `cursor`).
//! **Invariants:** read-only — every filter and page change is a link to a new viewer location,
//! so each view of the policy can be shared; a missing count or decision field renders as zero
//! or empty text instead of failing the page.
use super::super::{
    data::use_read,
    layout::{number, title, Feedback, Pager, Search},
    page::ViewerContext,
};
use crate::foundation::transport::dto::equipment_data_viewer::EquipmentSourcePage;
use leptos::prelude::*;

/// The selection tab of the gameplay dataset: why each reviewed native field is included in or
/// excluded from the gameplay catalog, filterable by decision kind and searchable.
#[component]
pub fn SelectionSummary() -> impl IntoView {
    let context = expect_context::<ViewerContext>();
    let data = use_read::<EquipmentSourcePage>(Memo::new(move |_| {
        let nav = context.nav.get();
        nav.request(
            "selection",
            &context.generation.get(),
            &[
                ("q", nav.get("selection_q")),
                ("cursor", nav.get("cursor")),
                ("kind", nav.get("selection_kind")),
            ],
        )
    }));
    view! {<section class="dv-overview"><div class="dv-eyebrow">"GAMEPLAY SELECTION POLICY"</div><h2>"What is included, and why"</h2>
        <p class="dv-muted">"Every reviewed native field has an explicit decision. Excluded fields are omitted from this catalog; they are not missing from the game. Full source configurations remain available in diagnostics."</p>
        <div class="dv-filters">{[("","All decisions"),("retain_value","Values"),("retain_relationship","Relationships"),("traverse_required_container","Container traversal"),("exclude","Excluded")].into_iter().map(|(kind,label)|view!{<a href=move||context.nav.get().href(&[("selection_kind",kind),("cursor","")])>{label}</a>}).collect_view()}</div>
        <Search key="selection_q" placeholder="Find a class, property, purpose or reason"/>
        <Feedback error=data.error loading=data.loading/>
        {move||data.value.get().map(|page|{
            let metadata:serde_json::Value=serde_json::from_str(&page.node_metadata_json).unwrap_or_default();
            let counts=metadata["counts"].as_object().cloned().unwrap_or_default();
            view!{<div class="dv-cards">{counts.into_iter().map(|(kind,count)|view!{<div class="dv-card"><span>{title(&kind)}</span><strong>{number(count.as_u64().unwrap_or(0))}</strong></div>}).collect_view()}</div>
                {metadata["has_publication_receipt"].as_bool().unwrap_or(false).then(||{
                    let excluded=number(metadata["excluded_dependency_count"].as_u64().unwrap_or(0));
                    view!{<p>{excluded}" diagnostic-only dependencies excluded. "<a href=move||context.nav.get().href(&[("tab","document"),("document","publication_receipt"),("pointer",""),("cursor","")])>"Size breakdown and excluded resource inventory →"</a></p>}
                })}
                <div class="dv-selection-list">{page.items.into_iter().map(|item|{
                    let decision:serde_json::Value=serde_json::from_str(&item.value_json).unwrap_or_default();
                    view!{<article class="dv-selection-row"><strong>{item.label}</strong><span>{format!("{} · {} · {}",decision["native_type"].as_str().unwrap_or(""),title(decision["disposition"].as_str().unwrap_or("")),title(decision["section"].as_str().unwrap_or("")))}</span><p>{decision["reason"].as_str().unwrap_or("").to_owned()}</p></article>}
                }).collect_view()}</div><Pager total=page.total next=page.next_cursor/>
            }
        })}
    </section>}
}
