//! Paged card batches and exact-source jumps share the grid cache.
use super::super::{data::use_read, page::ViewerContext};
use super::card_grid::GridContext;
use crate::foundation::transport::dto::equipment_data_viewer::EquipmentResourceCardPage;
use leptos::prelude::*;
/// Stores a loaded card page in `grid`: updates the card total, clears that page's error, and
/// keeps at most four pages by evicting those farthest from the new one.
pub fn accept(grid: GridContext, page: EquipmentResourceCardPage) {
    grid.total.set(page.total as usize);
    let start = page.start_index as usize;
    grid.pages.update(|pages| {
        pages.insert(start, page);
        while pages.len() > 4 {
            let first = *pages.keys().max_by_key(|k| k.abs_diff(start)).unwrap();
            pages.remove(&first);
        }
    });
    grid.errors.update(|errors| {
        errors.remove(&start);
    });
}
/// Loads the card page starting at `offset`, unless the grid already holds it, and stores the page
/// or its error in the grid. It renders nothing.
#[component]
pub fn BatchLoader(offset: usize) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let grid = expect_context::<GridContext>();
    let existing = grid
        .pages
        .with_untracked(|pages| pages.contains_key(&offset));
    let read = use_read::<EquipmentResourceCardPage>(Memo::new(move |_| {
        if existing {
            String::new()
        } else {
            let n = c.nav.get();
            n.inspection_request(
                "resource-cards",
                &c.generation.get(),
                &[
                    ("cursor", &offset.to_string()),
                    ("q", n.get("data_q")),
                    ("retry", &grid.retry.get().to_string()),
                ],
            )
        }
    }));
    Effect::new(move |_| {
        if let Some(page) = read.value.get() {
            accept(grid, page);
        }
        if let Some(error) = read.error.get() {
            grid.errors.update(|errors| {
                errors.insert(offset, error);
            });
        }
    });
}
/// Brings the container the location names into view: loads its card page, expands the named
/// property, and scrolls the grid to its row. When the container is not in the current
/// configuration view, it offers a link to look in all configurations.
#[component]
pub fn FocusCard(focus: Memo<(String, String)>) -> impl IntoView {
    let c = expect_context::<ViewerContext>();
    let grid = expect_context::<GridContext>();
    let read = use_read::<EquipmentResourceCardPage>(Memo::new(move |_| {
        let (node, _) = focus.get();
        if node.is_empty() {
            String::new()
        } else {
            c.nav.get().inspection_request(
                "resource-cards",
                &c.generation.get(),
                &[("node_id", &node)],
            )
        }
    }));
    Effect::new(move |_| {
        let (node, property) = focus.get();
        if node.is_empty() {
            return;
        }
        if let Some(page) = read.value.get() {
            if let Some(card) = page.items.iter().find(|card| card.node_id == node) {
                let index = card.index as usize;
                if !property.is_empty() {
                    grid.memory.update(|m| {
                        m.expanded.insert((node.clone(), property.clone()));
                    });
                }
                accept(grid, page);
                let row = index / grid.columns.get_untracked();
                let top = grid.layout.with_untracked(|l| l.top(row));
                if let Some(root) = grid.root.get_untracked() {
                    root.set_scroll_top(top as i32);
                    grid.scroll.set(top);
                }
            }
        }
    });
    view! {<Show when=move||read.error.get().is_some()><div class="dv-grid-message">{move||read.error.get()}<a href=move||c.nav.get().href(&[("view","all"),("data_q","")])>"Look in all configurations"</a></div></Show>}
}
