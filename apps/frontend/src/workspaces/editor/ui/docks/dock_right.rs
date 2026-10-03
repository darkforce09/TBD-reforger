//! Right-dock palette, selection routing, and authored asset controls.
//!
//! The dock groups Factions, Vehicles, Zones, Compositions, Triggers, Favourites, and Markers.
//! Palette leaves arm placement; side chips select the active faction or Objects mode.
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::bridge::host_state::editor_context;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use map_editing_tools::selection;
#[cfg(target_arch = "wasm32")]
use mission_editing_commands::hosted_commands as engine_ops;

use serde::{Deserialize, Serialize};

use crate::foundation::transport::dto::RegistryItem;
#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::MaterialIcon;
use crate::workspaces::editor::arsenal::asset_catalog::CatalogPalette;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::arsenal::asset_catalog::{CatalogNode, CatalogState};
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::bridge::host_state::armed_placement;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::session::layout::{DOCK_R, STUB_PX};
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::ui::docks::dock_left::collapse_chevron;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::ui::inspector::zones_panel::zones_panel;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::ui::outliner::tree::PALETTE_LEAF;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::ui::outliner::tree::{chevron_or_spacer, guide_spans};

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
mod shell;
mod triggers;

#[cfg(target_arch = "wasm32")]
use eden::*;
use favourites::*;
#[cfg(test)]
use markers::*;
#[cfg(target_arch = "wasm32")]
use palette::*;
use recent::*;
#[cfg(test)]
use shell::*;

#[cfg(target_arch = "wasm32")]
pub(crate) use compositions::compositions_panel;
#[cfg(test)]
pub use eden::SEARCH_GRAMMAR_HINT;
pub use eden::{
    apply_eden_chip, custom_chip_visible, eden_chip_selected, EdenChip, EdenSubmode,
    EDEN_CUSTOM_CHIP, EDEN_SIDE_CHIPS, SEARCH_PLACEHOLDER_GRAMMAR,
};
#[cfg(target_arch = "wasm32")]
pub use favourites::load_favourites;
pub use favourites::Favourites;
#[cfg(test)]
pub use favourites::{resolve_favourites, FavouriteAsset, FavouriteRow};
pub use markers::marker_icon_is_authorable;
#[cfg(target_arch = "wasm32")]
pub(crate) use markers::markers_panel;
#[cfg(test)]
pub use markers::{default_marker_icon, filter_marker_icons, marker_icons};
#[cfg(target_arch = "wasm32")]
pub use palette::PaletteKind;
#[cfg(target_arch = "wasm32")]
pub(crate) use recent::record_placed;
pub use recent::RecentPlaced;
pub(crate) use shell::route_select_zone;
#[cfg(target_arch = "wasm32")]
pub use shell::DockRight;
#[cfg(test)]
pub(crate) use shell::{install_select_zone, register_select_zone, ZONES_TAB};
#[cfg(target_arch = "wasm32")]
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
