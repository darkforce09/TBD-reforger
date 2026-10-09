//! Right-dock palette, selection routing, and authored asset controls.
//!
//! The dock groups Factions, Vehicles, Zones, Compositions, Triggers, Favourites, and Markers.
//! Palette leaves arm placement; side chips select the active faction or Objects mode.
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use map_editing_tools::selection;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::host_state::editor_context;
#[cfg(target_arch = "wasm32")]
use mission_editing_commands::hosted_commands as engine_ops;

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use crate::ui::docks::dock_left::collapse_chevron;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::zones_panel::zones_panel;
#[cfg(target_arch = "wasm32")]
use crate::ui::outliner::tree::PALETTE_LEAF;
#[cfg(target_arch = "wasm32")]
use crate::ui::outliner::tree::{chevron_or_spacer, guide_spans};
#[cfg(any(test, target_arch = "wasm32"))]
use frontend_api_dtos::RegistryItem;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::host_state::armed_placement;
#[cfg(any(test, target_arch = "wasm32"))]
use mission_creator_state::asset_catalog::CatalogPalette;
#[cfg(target_arch = "wasm32")]
use mission_creator_state::asset_catalog::{CatalogNode, CatalogState};
#[cfg(target_arch = "wasm32")]
use mission_creator_state::layout::{DOCK_R, STUB_PX};

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

mod compositions;
mod eden;
mod favourites;
mod markers;
mod palette;
mod recent;
pub mod shell;
pub mod triggers;

#[cfg(target_arch = "wasm32")]
use eden::*;
#[cfg(any(test, target_arch = "wasm32"))]
use favourites::*;
#[cfg(target_arch = "wasm32")]
use palette::*;
#[cfg(any(test, target_arch = "wasm32"))]
use recent::*;

#[cfg(target_arch = "wasm32")]
pub(crate) use compositions::compositions_panel;
pub use eden::{
    EDEN_CUSTOM_CHIP, EDEN_SIDE_CHIPS, EdenChip, EdenSubmode, SEARCH_PLACEHOLDER_GRAMMAR,
    apply_eden_chip, custom_chip_visible, eden_chip_selected,
};
pub use favourites::Favourites;
#[cfg(target_arch = "wasm32")]
pub use favourites::load_favourites;
#[cfg(test)]
pub(crate) use favourites::{FavouriteAsset, FavouriteRow, resolve_favourites};
#[cfg(target_arch = "wasm32")]
pub(crate) use markers::markers_panel;
#[cfg(test)]
use mission_creator_state::marker_icons::{
    CANONICAL_MARKER_SLUGS, MISSION_SCHEMA_JSON, default_marker_icon, filter_marker_icons,
    marker_icon_is_authorable, marker_icons,
};
#[cfg(target_arch = "wasm32")]
pub use palette::PaletteKind;
pub use recent::RecentPlaced;
#[cfg(target_arch = "wasm32")]
pub use shell::DockRight;
#[cfg(test)]
pub(crate) use shell::register_select_zone;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use shell::route_select_zone;
#[cfg(target_arch = "wasm32")]
pub(crate) use triggers::triggers_panel;
#[cfg(test)]
#[path = "tests/dock_right/compositions.rs"]
mod compositions_tests;

#[cfg(test)]
#[path = "tests/dock_right/favourites_and_recent_placements.rs"]
mod favourites_and_recent_placements_tests;

#[cfg(test)]
#[path = "tests/dock_right/marker_icons_and_briefing.rs"]
mod marker_icons_and_briefing_tests;

#[cfg(test)]
#[path = "tests/dock_right/triggers_and_owner_links.rs"]
mod triggers_and_owner_links_tests;

#[cfg(test)]
#[path = "tests/dock_right/zone_selection_seam.rs"]
mod t754_zone_selection_seam;
