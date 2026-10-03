//! Each native container owns its field continuation and expanded evidence.
use super::super::{data::use_read, page::ViewerContext, source_inspector::metadata};
use super::{card_grid::GridContext, field_row::FieldRow, observers};
use crate::foundation::transport::dto::equipment_data_viewer::{
    EquipmentResourceCard, EquipmentSourcePage, EquipmentSourcePageItemsItem,
};
use leptos::prelude::*;
/// The card of the container at grid position `index`: a loading message with a retry button
/// until its page arrives, then the container's header and fields.
#[component]
pub fn ComponentCard(index: usize) -> impl IntoView {
    let grid = expect_context::<GridContext>();
    let card = Memo::new(move |_| {
        grid.pages.with(|pages| {
            pages
                .get(&(index / 12 * 12))
                .and_then(|p| p.items.iter().find(|c| c.index as usize == index))
                .cloned()
        })
    });
    let available = Memo::new(move |_| {
        card.with(|c| {
            c.as_ref()
                .map(|c| c.node_id.clone())
                .into_iter()
                .collect::<Vec<_>>()
        })
    });
    view! {<article class="dv-component-card" id=format!("dv-card-{index}") data-card-index=index>
        <Show when=move||available.get().is_empty()><p class="dv-muted">{move||grid.errors.with(|errors|errors.get(&(index/12*12)).cloned()).unwrap_or_else(||"Loading container…".into())}</p><button on:click=move|_|{grid.pages.update(|p|{p.remove(&(index/12*12));});grid.retry.update(|v|*v+=1);}>"Retry"</button></Show>
        <For each=move||available.get() key=|id|id.clone() children=move |_|view!{<CardFields card=card.get_untracked().unwrap()/>}/>
    </article>}
}
/// The body of a loaded container card: capability and view tags, identity and inheritance
/// metadata with parent and child links, the property the location names when it is not loaded
/// yet, and the fields, whose next pages load as the card's end nears the viewport or on request.
#[component]
fn CardFields(card: EquipmentResourceCard) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let grid = expect_context::<GridContext>();
    let source = StoredValue::new(card.clone());
    let node = StoredValue::new(card.node_id.clone());
    let facts = RwSignal::new(card.facts);
    let next = RwSignal::new(card.property_next_cursor);
    let pending = RwSignal::new(false);
    let cursor = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let retry = RwSignal::new(0u64);
    let sentinel = NodeRef::<leptos::html::Div>::new();
    let near = RwSignal::new(false);
    observers::near(sentinel, grid.root, near);
    let read = use_read::<EquipmentSourcePage>(Memo::new(move |_| {
        cursor
            .get()
            .map(|offset| {
                c.nav.get().inspection_request(
                    "properties",
                    &c.generation.get(),
                    &[
                        ("node_id", &node.get_value()),
                        ("cursor", &offset),
                        ("retry", &retry.get().to_string()),
                    ],
                )
            })
            .unwrap_or_default()
    }));
    Effect::new(move |_| {
        if near.get() && !pending.get() && error.get().is_none() {
            if let Some(offset) = next.get() {
                pending.set(true);
                cursor.set(Some(offset));
            }
        }
    });
    Effect::new(move |_| {
        if let Some(page) = read.value.get() {
            facts.update(|f| {
                for item in page.items {
                    if !f.iter().any(|old| old.key == item.key) {
                        f.push(item);
                    }
                }
            });
            next.set(page.next_cursor);
            pending.set(false);
        }
        if let Some(e) = read.error.get() {
            error.set(Some(e));
            pending.set(false);
        }
    });
    let focus = Memo::new(move |_| {
        let n = c.nav.get();
        if n.get("node") == node.get_value() {
            n.get("property").to_owned()
        } else {
            String::new()
        }
    });
    let exact = use_read::<EquipmentSourcePage>(Memo::new(move |_| {
        let property = focus.get();
        if property.is_empty() {
            String::new()
        } else {
            c.nav.get().inspection_request(
                "properties",
                &c.generation.get(),
                &[("node_id", &node.get_value()), ("property", &property)],
            )
        }
    }));
    let selected = Memo::new(move |_| {
        exact
            .value
            .get()
            .and_then(|p| p.items.into_iter().next())
            .filter(|f| !facts.with(|items| items.iter().any(|i| i.key == f.key)))
    });
    let meta: serde_json::Value = serde_json::from_str(&card.metadata_json).unwrap_or_default();
    let mut links = Vec::new();
    if let Some(id) = meta["ancestor_id"].as_str() {
        links.push(("Parent configuration".to_owned(), id.to_owned()));
    }
    if let Some(children) = meta["children"].as_array() {
        for (i, id) in children.iter().enumerate() {
            if let Some(id) = id.as_str() {
                links.push((format!("Child {}", i + 1), id.to_owned()));
            }
        }
    }
    view! {<header class="dv-component-heading"><div class="dv-tags">{card.capabilities.into_iter().map(|cap|view!{<span>{super::super::layout::title(&cap)}</span>}).collect_view()}<span>{card.view}</span></div><h3>{card.class_name}</h3><small>{card.instance_name}</small><details><summary>"Container identity & inheritance"</summary>{metadata(&card.metadata_json)}<div class="dv-native-links">{links.into_iter().map(|(label,id)|view!{<a href=c.nav.get_untracked().href(&[("node",&id),("property",""),("view","all"),("data_q","")])>{label}" →"</a>}).collect_view()}</div><a href=move||c.nav.get().href(&[("tab","document"),("document","source"),("pointer","")])>"Complete source document →"</a></details></header>
        {move||selected.get().map(|fact|view!{<div class="dv-focused-field"><small>"Linked source property"</small><FieldRow fact node=node.get_value() index=source.get_value().index as usize/></div>})}
        <div class="dv-card-fields"><For each=move||facts.get() key=|fact|fact.key.clone() children=move|fact:EquipmentSourcePageItemsItem|view!{<FieldRow fact node=node.get_value() index=source.get_value().index as usize/>}/></div>
        <div node_ref=sentinel class="dv-card-continuation"><small>{move||format!("{} / {} fields loaded",facts.with(|v|v.len()),source.get_value().property_count)}</small>
            <Show when=move||next.get().is_some()><button disabled=move||pending.get() on:click=move|_|{error.set(None);pending.set(true);cursor.set(next.get_untracked());retry.update(|r|*r+=1);}>{move||if pending.get(){"Loading fields…"}else{"Load more fields"}}</button></Show>
            {move||error.get().map(|message|view!{<p class="dv-muted">{message}</p><button on:click=move|_|{error.set(None);retry.update(|r|*r+=1);}>"Retry fields"</button>})}
        </div>
    }
}
