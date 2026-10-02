//! Source provenance and object links stay beside the expanded field.
use super::super::{
    page::ViewerContext,
    source_inspector::{display, metadata},
};
use super::value_details::ValueDetails;
use crate::v2::core::api::dto::equipment_data_viewer::EquipmentSourcePageItemsItem;
use leptos::prelude::*;
/// The expanded details of one field: status, origin and unit tags, the full value when it is
/// small, links to the containers it references, source and inheritance evidence, and in-place
/// value browsers for a large value and for expandable metadata.
#[component]
pub fn InlineDetails(fact: EquipmentSourcePageItemsItem, node: String) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let property = fact.key.clone();
    let big = !fact.expanded || matches!(fact.value_kind.as_str(), "array" | "object");
    view! {<section class="dv-inline-details"><div class="dv-tags"><span>{fact.status.clone()}</span><span>{fact.origin.clone()}</span><span>{fact.native_unit.clone().unwrap_or_else(||"Unit unspecified".into())}</span></div>
        {(!big).then(||view!{<pre class="dv-value-full">{display(&fact.value_json)}</pre>})}
        <div class="dv-native-links">{fact.links.into_iter().map(|link|view!{<a href=c.nav.get_untracked().href(&[("node",&link.node_id),("property",""),("view","all"),("section","data"),("data_q","")])>{link.label}" →"</a>}).collect_view()}</div>
        <details><summary>"Source, inheritance & enum evidence"</summary>{metadata(&fact.metadata_json)}</details>
        {big.then(||view!{<ValueDetails node=node.clone() property=property.clone()/>})}
        {fact.metadata_json.contains("expand_metadata").then(||view!{<ValueDetails node=node.clone() property=property.clone() metadata=true/>})}
    </section>}
}
