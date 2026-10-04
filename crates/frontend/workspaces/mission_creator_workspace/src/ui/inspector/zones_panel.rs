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
    MIN_AUTHORABLE_RADIUS_M, ZONE_GRID_M, ZoneRuleKind, humanize_key, humanize_token,
    polygon_is_committable, zone_rule_fields, zone_types,
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
use mission_operations::zones::ZoneShape;

#[cfg(test)]
fn zones_panel_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| {
        [
    include_str!("zones_panel/zone_attributes.rs"),
    frontend_test_support::repository_root::repository_text(
        env!("CARGO_MANIFEST_DIR"),
        "crates/frontend/workspaces/mission_creator_state/src/zones/zone_geometry.rs",
    ),
    include_str!("zones_panel/zone_list_panel.rs"),
    include_str!("zones_panel/zone_rule_control.rs"),
    frontend_test_support::repository_root::repository_text(
        env!("CARGO_MANIFEST_DIR"),
        "crates/frontend/workspaces/mission_creator_state/src/zones/zone_schema_vocabulary.rs",
    ),
    include_str!("zones_panel.rs"),
]
        .concat()
    })
}

#[cfg(test)]
#[path = "tests/zones_panel/tactical_draw_trigger.rs"]
mod tactical_draw_trigger_tests;
#[cfg(test)]
#[path = "tests/zones_panel/zone_geometry_and_schema.rs"]
mod zone_geometry_and_schema_tests;
