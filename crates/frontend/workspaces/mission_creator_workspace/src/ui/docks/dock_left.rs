//! Editor layers, locations, bookmarks, and document search in the left dock.
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
/// Searchable document entity shared with the map engine index.
pub(crate) use mission_operations::document_index::DocEntity;
/// Kind of searchable document entity.
pub(crate) use mission_operations::document_index::DocKind;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use mission_creator_state::layout::{DOCK_L, STUB_PX};

/// qualifier bought nothing: the dock holds exactly one kind of layer.
#[cfg(any(test, target_arch = "wasm32"))]
const TAB_LABEL_LAYERS: &str = "Layers";
#[cfg(any(test, target_arch = "wasm32"))]
const TAB_LABEL_PLACES: &str = "Locations";
/// character's advance, in CSS px.
///
/// MEASURED, not guessed: rendering the real classes against the generated `aegis.css` in a headless
/// Chrome gives 45.75 px for the 6 characters of "Layers" (7.63/char) and 72.88 px for the 9 of
/// "Locations" (8.10/char), in DejaVu Sans — the widest fallback in the stack. Inter and system-ui,
/// which is what actually renders, are both narrower, so 8.5 is a genuine ceiling with margin.
#[cfg(test)]
const UPPERCASE_LABEL_ADVANCE_PX: f64 = 8.5;
#[cfg(test)]
const TAB_LABEL_PAD_PX: f64 = 12.0;
#[cfg(target_arch = "wasm32")]
use crate::ui::outliner::tree::virtual_tree;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::host_state::entity_selection;
use mission_creator_state::outliner_model::OutlinerNode;
#[cfg(target_arch = "wasm32")]
use mission_editing_commands::hosted_commands as engine_ops;

/// EXPANDED (points out of the dock — `chevron_left` for the left dock, `chevron_right` for the
/// right); the collapsed state shows the OTHER chevron in the SAME 24×24 box (the "flip the glyph"
/// rule). `at_start` places it at the row's start (left dock, outer corner = top-left) vs end (right
/// dock, top-right). The button flips `collapsed`; `mission_editor` observes that signal to mirror the
/// [`mission_creator_state::layout`] inset latch + run the reflow/centre-hold, so the chevron itself stays a pure
/// toggle.
#[cfg(target_arch = "wasm32")]
pub fn collapse_chevron(collapsed: RwSignal<bool>, expanded_is_left: bool) -> impl IntoView {
    let title = move || {
        if collapsed.get() {
            "Expand panel"
        } else {
            "Collapse panel"
        }
    };
    let icon = move || match (collapsed.get(), expanded_is_left) {
        (false, true) => "chevron_left",   // left dock, expanded: « outward
        (true, true) => "chevron_right",   // left dock, collapsed: » (expand)
        (false, false) => "chevron_right", // right dock, expanded: » outward
        (true, false) => "chevron_left",   // right dock, collapsed: « (expand)
    };
    view! {
        <button
            type="button"
            aria-label=title
            aria-expanded=move || (!collapsed.get()).to_string()
            title=title
            class="flex size-6 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
            on:click=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                collapsed.update(|c| *c = !*c);
            }
        >
            {move || view! { <MaterialIcon name=icon() class="block text-base" /> }}
        </button>
    }
}

mod bookmarks;
mod camera;
mod document_search;
mod places;
pub mod view;

#[cfg(test)]
use bookmarks::*;
/// Bookmark storage and filtering used by the Locations tab.
pub use bookmarks::{Bookmarks, default_bookmark_name, filter_bookmarks};
#[cfg(target_arch = "wasm32")]
pub use bookmarks::{load_bookmarks, save_bookmarks};
#[cfg(target_arch = "wasm32")]
use camera::*;
/// Camera snapshot and movement helpers for location rows.
#[cfg(target_arch = "wasm32")]
pub use camera::{fly_to, live_camera};
#[cfg(test)]
use document_search::*;
/// Document search results and selection controls.
pub use document_search::{
    DocHit, MAX_DOC_HITS, hit_is_routable, query_hits, search_document, selection_facets,
    unselectable_reason,
};
#[cfg(target_arch = "wasm32")]
pub use document_search::{SelectionFacet, apply_selection, document_rows, selection_rows};
/// Layer and named-location filtering helpers.
pub use places::{
    LeftTab, NamedPlace, filter_outliner, filter_places, find_layer_label, first_folder_label,
    matches_query, sort_places,
};
/// Left editor dock component.
#[cfg(target_arch = "wasm32")]
pub use view::DockLeft;

#[cfg(test)]
#[path = "tests/dock_left/bookmarks_places_and_tabs.rs"]
mod bookmarks_places_and_tabs;

#[cfg(test)]
#[path = "tests/dock_left/dock_density_and_search.rs"]
mod dock_density_and_search;

#[cfg(test)]
#[path = "tests/dock_left/document_search.rs"]
mod document_search_tests;

#[cfg(test)]
#[path = "tests/dock_left/test_source.rs"]
mod test_source;
