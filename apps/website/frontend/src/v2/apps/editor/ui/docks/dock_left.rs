//! Editor layers, locations, bookmarks, and document search in the left dock.
#![allow(dead_code)]
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
/// Searchable document entity shared with the map engine index.
pub use website_map_engine::data::store::operations::document_index::DocEntity;
/// Kind of searchable document entity.
pub use website_map_engine::data::store::operations::document_index::DocKind;

use crate::v2::apps::editor::shell::layout::{DOCK_L, STUB_PX};

/// qualifier bought nothing: the dock holds exactly one kind of layer.
const TAB_LABEL_LAYERS: &str = "Layers";
const TAB_LABEL_PLACES: &str = "Locations";
/// character's advance, in CSS px.
///
/// MEASURED, not guessed: rendering the real classes against the generated `aegis.css` in a headless
/// Chrome gives 45.75 px for the 6 characters of "Layers" (7.63/char) and 72.88 px for the 9 of
/// "Locations" (8.10/char), in DejaVu Sans — the widest fallback in the stack. Inter and system-ui,
/// which is what actually renders, are both narrower, so 8.5 is a genuine ceiling with margin.
const UPPERCASE_LABEL_ADVANCE_PX: f64 = 8.5;
const TAB_LABEL_PAD_PX: f64 = 12.0;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::entity_selection;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::outliner::outliner;
use crate::v2::apps::editor::ui::outliner::outliner::OutlinerNode;
use crate::v2::apps::editor::ui::outliner::tree::virtual_tree;
use crate::v2::core::ui::MaterialIcon;
use website_map_engine::editing::hosted_commands as engine_ops;

/// EXPANDED (points out of the dock — `chevron_left` for the left dock, `chevron_right` for the
/// right); the collapsed state shows the OTHER chevron in the SAME 24×24 box (the "flip the glyph"
/// rule). `at_start` places it at the row's start (left dock, outer corner = top-left) vs end (right
/// dock, top-right). The button flips `collapsed`; `mission_editor` observes that signal to mirror the
/// [`crate::v2::apps::editor::shell::layout`] inset latch + run the reflow/centre-hold, so the chevron itself stays a pure
/// toggle.
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
mod view;

use bookmarks::*;
/// Bookmark storage and filtering used by the Locations tab.
pub use bookmarks::{
    default_bookmark_name, filter_bookmarks, load_bookmarks, save_bookmarks, Bookmark, Bookmarks,
};
use camera::*;
/// Camera snapshot and movement helpers for location rows.
pub use camera::{fly_to, live_camera};
use document_search::*;
/// Document search results and selection controls.
pub use document_search::{
    apply_selection, document_rows, hit_is_routable, query_hits, search_document, selection_facets,
    selection_rows, unselectable_reason, DocHit, SelectionFacet, MAX_DOC_HITS,
};
use places::*;
/// Layer and named-location filtering helpers.
pub use places::{
    filter_outliner, filter_places, find_layer_label, first_folder_label, matches_query,
    sort_places, LeftTab, NamedPlace,
};
/// Left editor dock component.
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
