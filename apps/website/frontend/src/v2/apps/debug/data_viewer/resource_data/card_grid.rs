//! Windowed card rows retain their anchors as content grows.
use super::super::{
    browsing_state::{self, ResourceMemory},
    page::ViewerContext,
};
use super::{component_card::ComponentCard, grid_layout::RowLayout, grid_loading, observers};
use crate::v2::core::api::dto::equipment_data_viewer::EquipmentResourceCardPage;
use leptos::prelude::*;
use std::collections::BTreeMap;
#[derive(Clone, Copy)]
pub struct GridContext {
    pub root: NodeRef<leptos::html::Div>,
    pub scroll: RwSignal<f64>,
    pub height: RwSignal<f64>,
    pub columns: RwSignal<usize>,
    pub total: RwSignal<usize>,
    pub layout: RwSignal<RowLayout>,
    pub memory: RwSignal<ResourceMemory>,
    pub pages: RwSignal<BTreeMap<usize, EquipmentResourceCardPage>>,
    pub retry: RwSignal<u64>,
    pub errors: RwSignal<BTreeMap<usize, String>>,
}
#[component]
pub fn CardGrid(memory_key: String) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let key = StoredValue::new(memory_key);
    let restored = browsing_state::restore(&key.get_value());
    let grid = GridContext {
        root: NodeRef::new(),
        scroll: RwSignal::new(restored.scroll_top),
        height: RwSignal::new(700.),
        columns: RwSignal::new(restored.columns.max(1)),
        total: RwSignal::new(0),
        layout: RwSignal::new(RowLayout {
            heights: restored.row_heights.clone(),
        }),
        memory: RwSignal::new(restored),
        pages: RwSignal::new(BTreeMap::new()),
        retry: RwSignal::new(0),
        errors: RwSignal::new(BTreeMap::new()),
    };
    provide_context(grid);
    observers::size(
        grid.root,
        Callback::new(move |(width, height)| {
            let columns = if width >= 1250. {
                3
            } else if width >= 740. {
                2
            } else {
                1
            };
            if columns != grid.columns.get_untracked() {
                grid.columns.set(columns);
                grid.layout.set(RowLayout::default());
                grid.memory.update(|m| {
                    m.row_heights.clear();
                    m.columns = columns;
                });
            }
            grid.height.set(height);
        }),
    );
    let restored_position = RwSignal::new(false);
    Effect::new(move |_| {
        if grid.total.get() > 0 && !restored_position.get_untracked() {
            if let Some(root) = grid.root.get() {
                root.set_scroll_top(grid.scroll.get_untracked() as i32);
                restored_position.set(true);
            }
        }
    });
    Effect::new(move |_| {
        browsing_state::save(key.get_value(), grid.memory.get());
    });
    let rows = Memo::new(move |_| {
        grid.layout.with(|l| {
            l.visible(
                grid.scroll.get(),
                grid.height.get(),
                grid.total.get().div_ceil(grid.columns.get()),
            )
        })
    });
    let batches = Memo::new(move |_| {
        let cols = grid.columns.get();
        let mut pages = std::collections::BTreeSet::new();
        for r in rows.get() {
            for i in r * cols..((r + 1) * cols).min(grid.total.get()) {
                pages.insert(i / 12 * 12);
            }
        }
        if pages.is_empty() {
            pages.insert(0);
        }
        pages
            .into_iter()
            .map(|p| (p, grid.retry.get()))
            .collect::<Vec<_>>()
    });
    let focus = Memo::new(move |_| {
        let n = c.nav.get();
        (n.get("node").to_owned(), n.get("property").to_owned())
    });
    view! {<div class="dv-card-scroll" node_ref=grid.root tabindex="0" aria-label="Resource data" on:scroll=move|_|{if let Some(root)=grid.root.get(){let top=root.scroll_top().max(0) as f64;grid.scroll.set(top);grid.memory.update(|m|m.scroll_top=top);}}>
        <grid_loading::FocusCard focus/>
        <For each=move||batches.get() key=|key|*key children=move|(offset,_)|view!{<grid_loading::BatchLoader offset/>}/>
        <Show when=move||grid.total.get()==0><p class="dv-grid-message">{move||grid.errors.get().values().next().cloned().unwrap_or_else(||if grid.pages.get().is_empty(){"Loading source containers…".into()}else{"No matching containers.".into()})}<button on:click=move|_|grid.retry.update(|v|*v+=1)>"Retry"</button></p></Show>
        <div class="dv-card-canvas" style:height=move||format!("{}px",grid.layout.with(|l|l.top(grid.total.get().div_ceil(grid.columns.get()))))>
            <For each=move||rows.get() key=move|r|(*r,grid.columns.get_untracked()) children=move|row|view!{<CardRow row/>}/>
        </div>
    </div>}
}
#[component]
fn CardRow(row: usize) -> impl IntoView {
    let grid = expect_context::<GridContext>();
    let node = NodeRef::<leptos::html::Div>::new();
    observers::size(
        node,
        Callback::new(move |(_, height): (f64, f64)| {
            if height < 10. {
                return;
            }
            let cols = grid.columns.get_untracked();
            if !(row * cols..((row + 1) * cols).min(grid.total.get_untracked())).all(|i| {
                grid.pages
                    .with_untracked(|p| p.contains_key(&(i / 12 * 12)))
            }) {
                return;
            }
            let old = grid.layout.get_untracked();
            let delta = height - old.height(row);
            if delta.abs() < 1. {
                return;
            }
            let above = old.top(row) + old.height(row) < grid.scroll.get_untracked();
            grid.layout.update(|l| {
                l.heights.insert(row, height);
            });
            grid.memory.update(|m| {
                m.row_heights.insert(row, height);
            });
            if above {
                if let Some(root) = grid.root.get_untracked() {
                    let top = grid.scroll.get_untracked() + delta;
                    root.set_scroll_top(top as i32);
                    grid.scroll.set(top);
                }
            }
        }),
    );
    let indices = Memo::new(move |_| {
        let cols = grid.columns.get();
        (row * cols..((row + 1) * cols).min(grid.total.get())).collect::<Vec<_>>()
    });
    view! {<div node_ref=node class="dv-card-row" style:top=move||format!("{}px",grid.layout.with(|l|l.top(row))) style:grid-template-columns=move||format!("repeat({},minmax(0,1fr))",grid.columns.get())>
        <For each=move||indices.get() key=|i|*i children=move|index|view!{<ComponentCard index/>}/>
    </div>}
}
