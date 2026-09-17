//! Right-dock palette, selection routing, and authored asset controls.
//!
//! The dock groups Factions, Vehicles, Zones, Compositions, Triggers, Favourites, and Markers.
//! Palette leaves arm placement; side chips select the active faction or Objects mode.
#![allow(dead_code)]
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::editor_context;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::hosted_commands as engine_ops;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::tools::selection;

use serde::{Deserialize, Serialize};

use crate::v2::apps::editor::arsenal::asset_catalog::{CatalogNode, CatalogPalette, CatalogState};
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::armed_placement;
use crate::v2::apps::editor::shell::layout::{DOCK_R, STUB_PX};
use crate::v2::apps::editor::ui::docks::dock_left::collapse_chevron;
use crate::v2::apps::editor::ui::inspector::zones_panel::zones_panel;
use crate::v2::apps::editor::ui::outliner::tree::{chevron_or_spacer, guide_spans, PALETTE_LEAF};
use crate::v2::core::api::dto::RegistryItem;
use crate::v2::core::ui::MaterialIcon;

/// The vehicle-crew checkbox updates the placement preference used by the next vehicle drop.
#[cfg(target_arch = "wasm32")]
fn crew_place_toggle(with_crew: RwSignal<bool>) -> impl IntoView {
    view! {
        <label class="mt-2 flex items-center gap-2 text-label-sm text-on-surface-variant">
            <input
                type="checkbox"
                class="size-3.5 shrink-0 accent-primary"
                aria-label="Place vehicle with crew"
                prop:checked=move || with_crew.get()
                on:change=move |ev| {
                    let on = event_target_checked(&ev);
                    with_crew.set(on);
                    editor_context::set_place_with_crew(on);
                }
            />
            <span>"Place with crew"</span>
        </label>
    }
}

/// Native shell: the placement preference lives in the wasm-only `editor_ops`, so there is nothing
/// to toggle — the toggle renders on the wasm build only. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
fn crew_place_toggle() -> impl IntoView {
    ().into_view()
}

mod compositions;
mod eden;
mod favourites;
mod markers;
mod palette;
mod recent;
mod shell;
mod triggers;

use compositions::*;
use eden::*;
use favourites::*;
use markers::*;
use palette::*;
use recent::*;
use shell::*;
use triggers::*;

pub(crate) use compositions::compositions_panel;
pub use eden::{
    apply_eden_chip, custom_chip_visible, eden_chip_selected, EdenChip, EdenSubmode,
    EDEN_CUSTOM_CHIP, EDEN_SIDE_CHIPS, SEARCH_GRAMMAR_HINT, SEARCH_PLACEHOLDER_GRAMMAR,
};
pub use favourites::{
    load_favourites, resolve_favourites, save_favourites, FavouriteAsset, FavouriteRow, Favourites,
};
pub(crate) use markers::markers_panel;
pub use markers::{
    default_marker_icon, filter_marker_icons, marker_icon_is_authorable, marker_icons,
};
pub use palette::PaletteKind;
pub(crate) use recent::record_placed;
pub use recent::RecentPlaced;
pub use shell::DockRight;
pub(crate) use shell::{
    install_select_zone, register_select_zone, route_select_zone, unregister_select_zone, ZONES_TAB,
};
pub(crate) use triggers::triggers_panel;
#[cfg(test)]
#[path = "tests/dock_right/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/dock_right/tab_strip_budget.rs"]
mod t637_tab_strip_budget;

#[cfg(test)]
#[path = "tests/dock_right/zone_selection_seam.rs"]
mod t754_zone_selection_seam;

#[cfg(test)]
#[path = "tests/dock_right/zone_hook_lifecycle.rs"]
mod f2_zone_hook_lifecycle;
