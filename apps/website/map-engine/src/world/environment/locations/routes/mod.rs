//! Role: Module boundary for environment/locations/routes.
//! Position: `world/environment/locations/routes` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

#![forbid(unsafe_code)]

/// Re-export `crate::world::environment::locations::route_placement::ROAD_CLASSES`.
pub use crate::world::environment::locations::route_placement::ROAD_CLASSES;

/// Re-export `crate::world::environment::locations::route_placement::ROAD_NAME_DECLUTTER_BASE_M`.
pub use crate::world::environment::locations::route_placement::ROAD_NAME_DECLUTTER_BASE_M;

/// Re-export `crate::world::environment::locations::route_placement::ROAD_NAME_LONG_SEGMENT_M`.
pub use crate::world::environment::locations::route_placement::ROAD_NAME_LONG_SEGMENT_M;

/// Re-export `crate::world::environment::locations::route_placement::ROAD_NAME_MAX_ON_SCREEN`.
pub use crate::world::environment::locations::route_placement::ROAD_NAME_MAX_ON_SCREEN;

/// Re-export `crate::world::environment::locations::route_placement::ROAD_NAME_MIN_ZOOM_HIGHWAY`.
pub use crate::world::environment::locations::route_placement::ROAD_NAME_MIN_ZOOM_HIGHWAY;

/// Re-export `crate::world::environment::locations::route_placement::ROAD_NAME_MIN_ZOOM_SECONDARY`.
pub use crate::world::environment::locations::route_placement::ROAD_NAME_MIN_ZOOM_SECONDARY;

/// Re-export `crate::world::environment::locations::route_placement::ROAD_NAME_OFFSET_M`.
pub use crate::world::environment::locations::route_placement::ROAD_NAME_OFFSET_M;

/// Re-export `crate::world::environment::locations::route_placement::ROAD_NAME_PERP_TOL_M`.
pub use crate::world::environment::locations::route_placement::ROAD_NAME_PERP_TOL_M;

/// Re-export `crate::world::environment::locations::route_placement::RoadLabelPlacement`.
pub use crate::world::environment::locations::route_placement::RoadLabelPlacement;

/// Re-export `crate::world::environment::locations::route_placement::build_road_label_draw_set`.
pub use crate::world::environment::locations::route_placement::build_road_label_draw_set;

/// Re-export `crate::world::environment::locations::route_placement::declutter_road_labels`.
pub use crate::world::environment::locations::route_placement::declutter_road_labels;

/// Re-export `crate::world::environment::locations::route_placement::declutter_road_labels_in_order`.
pub use crate::world::environment::locations::route_placement::declutter_road_labels_in_order;

/// Re-export `crate::world::environment::locations::route_placement::major_roads_covered`.
pub use crate::world::environment::locations::route_placement::major_roads_covered;

/// Re-export `crate::world::environment::locations::route_placement::place_road_labels`.
pub use crate::world::environment::locations::route_placement::place_road_labels;

/// Re-export `crate::world::environment::locations::route_placement::road_class_code`.
pub use crate::world::environment::locations::route_placement::road_class_code;

/// Re-export `crate::world::environment::locations::route_placement::road_class_name`.
pub use crate::world::environment::locations::route_placement::road_class_name;

/// Re-export `crate::world::environment::locations::route_placement::road_class_priority`.
pub use crate::world::environment::locations::route_placement::road_class_priority;

/// Re-export `crate::world::environment::locations::route_placement::road_class_visibility_floor`.
pub use crate::world::environment::locations::route_placement::road_class_visibility_floor;

/// Re-export `crate::world::environment::locations::route_placement::road_declutter_invariant_holds`.
pub use crate::world::environment::locations::route_placement::road_declutter_invariant_holds;

/// Re-export `crate::world::environment::locations::route_placement::road_entry_visible`.
pub use crate::world::environment::locations::route_placement::road_entry_visible;

/// Re-export `crate::world::environment::locations::route_placement::road_name_visible_for_class`.
pub use crate::world::environment::locations::route_placement::road_name_visible_for_class;

/// Re-export `crate::world::environment::locations::route_placement::road_placement_geometry_holds`.
pub use crate::world::environment::locations::route_placement::road_placement_geometry_holds;

/// Re-export `crate::world::environment::locations::route_labels::RoadNameEntry`.
pub use crate::world::environment::locations::route_labels::RoadNameEntry;

/// Re-export `crate::world::environment::locations::route_labels::RoadNamesFile`.
pub use crate::world::environment::locations::route_labels::RoadNamesFile;

/// Re-export `crate::world::environment::locations::route_labels::build_road_label_draw_set_from_archive`.
pub use crate::world::environment::locations::route_labels::build_road_label_draw_set_from_archive;

/// Re-export `crate::world::environment::locations::route_labels::parse_road_names_json`.
pub use crate::world::environment::locations::route_labels::parse_road_names_json;

/// Re-export `crate::world::environment::locations::route_labels::road_name_schema_holds`.
pub use crate::world::environment::locations::route_labels::road_name_schema_holds;

/// Re-export `crate::world::environment::locations::route_labels::road_names_from_archive`.
pub use crate::world::environment::locations::route_labels::road_names_from_archive;

/// Re-export `crate::world::environment::locations::route_labels::road_names_to_archive`.
pub use crate::world::environment::locations::route_labels::road_names_to_archive;

/// Re-export `crate::world::environment::locations::route_geometry::perpendicular_dist_to_polyline`.
pub use crate::world::environment::locations::route_geometry::perpendicular_dist_to_polyline;

/// Re-export `crate::world::environment::locations::route_geometry::placement_fractions`.
pub use crate::world::environment::locations::route_geometry::placement_fractions;

/// Re-export `crate::world::environment::locations::route_geometry::point_tangent_at_frac`.
pub use crate::world::environment::locations::route_geometry::point_tangent_at_frac;

/// Re-export `crate::world::environment::locations::route_geometry::polyline_length`.
pub use crate::world::environment::locations::route_geometry::polyline_length;

/// Re-export `crate::world::environment::locations::route_geometry::road_declutter_min_dist_m`.
pub use crate::world::environment::locations::route_geometry::road_declutter_min_dist_m;

/// Re-export `crate::world::environment::locations::route_geometry::upright_angle_deg`.
pub use crate::world::environment::locations::route_geometry::upright_angle_deg;
#[cfg(test)]
mod tests;
