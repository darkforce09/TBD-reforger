//! Zones panel model and exports.

#![allow(dead_code)]

use leptos::prelude::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub use website_map_engine::data::store::operations::zones::DrawTarget;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};
#[cfg(target_arch = "wasm32")]
use crate::v2::core::ui::MaterialIcon;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::armed_placement;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::tactical_graphics_authoring;

mod zone_attributes;
mod zone_geometry;
mod zone_list_panel;
mod zone_rule_control;
mod zone_schema_vocabulary;

#[cfg(target_arch = "wasm32")]
use zone_attributes::zone_attributes;
#[cfg(target_arch = "wasm32")]
pub use zone_geometry::add_whole_terrain_zone;
pub use zone_geometry::{
    circle_from_clicks, polygon_flat, polygon_is_committable, project_owner_line,
    radius_survives_compile, terrain_rect_is_authorable, terrain_rect_ring,
    whole_terrain_zone_type, ProjectedOwnerLine, ZoneShape, MIN_AUTHORABLE_RADIUS_M,
    WHOLE_TERRAIN_ZONE_LABEL, ZONE_GRID_M,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use zone_list_panel::zones_panel;
#[cfg(target_arch = "wasm32")]
use zone_rule_control::zone_rule_control;
pub(crate) use zone_schema_vocabulary::MISSION_SCHEMA;
pub use zone_schema_vocabulary::{
    humanize_key, humanize_token, zone_rule_fields, zone_types, ZoneRuleField, ZoneRuleKind,
};

/// Renders authored zones and draw controls, or an empty native stub.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn zones_panel(doc_tick: RwSignal<u64>, selected: RwSignal<Option<String>>) -> AnyView {
    let _ = (doc_tick, selected);
    ().into_any()
}

#[cfg(test)]
use zone_geometry::round_coord;

#[cfg(test)]
const ZONES_PANEL_SOURCE: &str = concat!(
    include_str!("zones_panel/zone_attributes.rs"),
    include_str!("zones_panel/zone_geometry.rs"),
    include_str!("zones_panel/zone_list_panel.rs"),
    include_str!("zones_panel/zone_rule_control.rs"),
    include_str!("zones_panel/zone_schema_vocabulary.rs"),
    include_str!("zones_panel.rs"),
);

#[cfg(test)]
#[path = "tests/zones_panel/tactical_draw_trigger.rs"]
mod tactical_draw_trigger_tests;
#[cfg(test)]
#[path = "tests/zones_panel/zone_geometry_and_schema.rs"]
mod zone_geometry_and_schema_tests;
