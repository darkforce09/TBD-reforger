//! Zones panel model and exports.

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub use mission_operations::zones::DrawTarget;

#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};

#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::bridge::host_state::armed_placement;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::bridge::tactical_graphics_authoring;

mod zone_attributes;
mod zone_geometry;
mod zone_list_panel;
mod zone_rule_control;
mod zone_schema_vocabulary;

#[cfg(target_arch = "wasm32")]
use zone_attributes::zone_attributes;
#[cfg(target_arch = "wasm32")]
pub use zone_geometry::add_whole_terrain_zone;
#[cfg(target_arch = "wasm32")]
pub use zone_geometry::ProjectedOwnerLine;
#[cfg(target_arch = "wasm32")]
pub use zone_geometry::ZoneShape;
pub use zone_geometry::{
    circle_from_clicks, polygon_flat, polygon_is_committable, project_owner_line,
    MIN_AUTHORABLE_RADIUS_M, ZONE_GRID_M,
};
#[cfg(test)]
pub use zone_geometry::{
    radius_survives_compile, terrain_rect_is_authorable, terrain_rect_ring,
    whole_terrain_zone_type, WHOLE_TERRAIN_ZONE_LABEL,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use zone_list_panel::zones_panel;
#[cfg(target_arch = "wasm32")]
use zone_rule_control::zone_rule_control;
#[cfg(target_arch = "wasm32")]
pub use zone_schema_vocabulary::ZoneRuleField;
pub(crate) use zone_schema_vocabulary::MISSION_SCHEMA;
pub use zone_schema_vocabulary::{
    humanize_key, humanize_token, zone_rule_fields, zone_types, ZoneRuleKind,
};

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
