//! Zones panel model and exports.

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use mission_operations::zones::DrawTarget;

#[cfg(target_arch = "wasm32")]
use crate::ui::outliner::tree::{ROW, ROW_ACTIVE};
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;

#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::host_state::armed_placement;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::tactical_graphics_authoring;

mod zone_attributes;
mod zone_list_panel;
mod zone_rule_control;

#[cfg(target_arch = "wasm32")]
use zone_attributes::zone_attributes;
#[cfg(target_arch = "wasm32")]
pub(crate) use zone_list_panel::zones_panel;
#[cfg(target_arch = "wasm32")]
use zone_rule_control::zone_rule_control;

#[cfg(any(test, target_arch = "wasm32"))]
use mission_creator_state::zones::{
    MIN_AUTHORABLE_RADIUS_M, ZONE_GRID_M, ZoneRuleKind, polygon_is_committable, zone_rule_fields,
    zone_types,
};
#[cfg(test)]
use mission_creator_state::zones::{
    MISSION_SCHEMA, WHOLE_TERRAIN_ZONE_LABEL, circle_from_clicks, polygon_flat,
    radius_survives_compile, round_coord, terrain_rect_is_authorable, terrain_rect_ring,
    whole_terrain_zone_type,
};
#[cfg(target_arch = "wasm32")]
use mission_creator_state::zones::{ZoneRuleField, add_whole_terrain_zone};
#[cfg(target_arch = "wasm32")]
use mission_creator_state::zones::{humanize_key, humanize_token};
#[cfg(target_arch = "wasm32")]
use mission_operations::zones::ZoneShape;

#[cfg(test)]
#[path = "tests/zones_panel/zone_geometry_and_schema.rs"]
mod zone_geometry_and_schema_tests;
