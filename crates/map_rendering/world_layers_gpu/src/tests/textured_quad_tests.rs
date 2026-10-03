//! A textured quad is measured from the scene anchor, and only the forest fill samples a density
//! raster.

use super::{textured_pipeline_for, world_rect_rel};
use map_draw_lanes::lane_roles::LaneRole;
use renderer_core::packet_bindings::{PIPE_DENSITY, PIPE_TEXTURED};

#[test]
fn the_rectangle_is_relative_to_the_scene_anchor() {
    assert_eq!(
        world_rect_rel([0.0, 0.0], [12_800.0, 12_800.0]),
        [-6400.0, -6400.0, 6400.0, 6400.0]
    );
    assert_eq!(
        world_rect_rel([6400.0, 6500.0], [6450.5, 6600.0]),
        [0.0, 100.0, 50.5, 200.0]
    );
}

#[test]
fn the_forest_fill_draws_with_the_density_pipeline() {
    assert_eq!(textured_pipeline_for(LaneRole::ForestFill), PIPE_DENSITY);
}

#[test]
fn every_other_textured_lane_draws_with_the_plain_textured_pipeline() {
    for role in [LaneRole::Satellite, LaneRole::Hillshade, LaneRole::Viewshed] {
        assert_eq!(textured_pipeline_for(role), PIPE_TEXTURED);
    }
}
