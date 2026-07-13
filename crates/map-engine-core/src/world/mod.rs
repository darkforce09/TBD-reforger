//! T-151.2 (W2) — world-object parser ported to Rust, proven byte-exact against the JS
//! `worldObjectsCore` oracle: SoA columns are **Class R** (bit-identical `Float32Array`/
//! `Uint16Array`/`Uint8Array` stores), per-render-class row sets are **Class S**, and the OBB
//! corners + road centerline are **Class T** (≤ 1 ULP). Pure compute — no `wasm-bindgen`, no
//! deck.gl; the JS boundary is the `map-engine-wasm` `WorldStore` handle.

mod airfield;
mod cartographic_strip;
mod chunk;
mod chunk_math;
mod classify;
mod density_ladder;
mod glyph_math;
mod importance_declutter;
mod index;
mod locations;
mod lod_gates;
mod manifest;
mod obb;
mod prefab;
mod regions;
mod residency;
mod road_labels;
mod roads;
mod store;

pub use airfield::{
    AIRFIELD_BBOX_MARGIN_M, APRON_AREA_MIN_M2, APRON_ELEV_TOLERANCE_M, APRON_FILL_RGBA,
    APRON_FLATNESS_SIGMA_M, RUNWAY_POLISH_WIDTH_M, apron_qualifying_area_m2,
    build_airfield_apron_mesh, compute_airfield_bbox, is_airfield_structure_class, point_in_bbox,
    polygon_area_m2,
};
pub use cartographic_strip::{
    BRIDGE_RAILING_RADIUS_M, FENCE_STRIP_RGBA, FENCE_STRIP_WIDTH_M, PIER_STRIP_MAX_WIDTH_M,
    STRIP_MIN_PX, clamp_strip_width_m, compose_bridge_rail_strips, compose_fence_strip,
    compose_pier_strip, obb_long_axis_endpoints, pack_cartographic_strips,
    strip_world_width_at_midpoint,
};
pub use chunk_math::{
    Bbox, ChunkRect, TerrainSizeM, chunk_id, chunk_ids_for_rect, chunk_ids_for_viewport,
    chunk_rect_for_bbox, expand_bbox, expand_chunk_rect, preload_margin_m,
};
pub use classify::{
    NO_CLASS, OVERSIZED_HALF_EXTENT_M, RENDER_CLASS_CODES, class_code, narrow_instance_row,
    render_class_for_prefab,
};
pub use density_ladder::{
    density_grid_dims, density_texel_sum_for_draw_ids, exact_tree_count, exact_tree_count_chunk,
    heatmap_trees, pack_density_grid_r32, parse_chunk_xy,
};
pub use glyph_math::{
    BADGE_BASE_SIZE_PX, BADGE_SIZE_MIN_PX, BUILDING_CLASSES, DEFAULT_BASE_SIZE_PX,
    DEFAULT_GLYPH_RGBA, GLYPH_SIZE_MIN_PX, ICON_INSTANCE_STRIDE, REF_TREE_HEIGHT_M, badge_icon_key,
    badge_size_meters, building_icon_key, deck_angle_for_rotation_deg, glyph_size_meters,
    hex_to_rgba, landmark_glyph_icon_key, pack_icon_instance, pack_rgba_u32, size_with_min_px,
    tree_size_multiplier, yaw_to_snorm16,
};
pub use importance_declutter::{
    IMPORTANCE_SCALE, LocationLabel, TOWN_BASE_SIZE_M, TOWN_LABEL_MAX_ZOOM, TOWN_LABEL_MIN_ZOOM,
    declutter_town_labels, nearest_more_important_m, should_draw_town_label, size_land_m,
    town_declutter_invariant_holds, town_declutter_threshold_m,
};
pub use index::WorldSpatialIndex;
pub use locations::{locations_to_label_specs, parse_locations_json};
pub use lod_gates::{
    BUILDING_BADGE_MIN_ZOOM, BUILDING_FOOTPRINT_MIN_ZOOM, FENCE_MIN_ZOOM, FOREST_FILL_MAX_ZOOM,
    FOREST_OUTLINE_MIN_ZOOM, INSTANCE_BUDGET, PIER_MIN_ZOOM, PROP_MIN_ZOOM, REF_ZOOM,
    ROCK_LARGE_MIN_ZOOM, SEA_FILL_MAX_ZOOM, TREE_GLYPH_MIN_ZOOM, VEGETATION_MIN_ZOOM,
    WORLD_RENDER_CLASSES, class_visible, contour_interval_for_zoom,
};
pub use manifest::{
    ChunkCell, DEFAULT_CHUNK_SIZE_M, ObjectsManifest, narrow_cells, parse_objects_manifest,
};
pub use obb::{
    BuildingPrefabInfo, FencePrefabInfo, building_prefab_lookup, fence_prefab_lookup, obb_corners,
};
pub use prefab::{PrefabEntry, PrefabRow, build_prefab_maps, narrow_prefab_rows};
pub use regions::{LandCoverRegion, parse_regions_payload};
pub use residency::{APPLY_BUDGET_MS, BUILDING_MIN_ZOOM, LRU_MIN_CHUNKS, WorldResidency};
pub use road_labels::{
    ROAD_NAME_DECLUTTER_BASE_M, ROAD_NAME_LONG_SEGMENT_M, ROAD_NAME_MAX_ON_SCREEN,
    ROAD_NAME_MIN_ZOOM_HIGHWAY, ROAD_NAME_MIN_ZOOM_SECONDARY, ROAD_NAME_OFFSET_M,
    ROAD_NAME_PERP_TOL_M, RoadLabelPlacement, RoadNameEntry, RoadNamesFile,
    build_road_label_draw_set, declutter_road_labels, major_roads_covered, parse_road_names_json,
    perpendicular_dist_to_polyline, place_road_labels, placement_fractions, point_tangent_at_frac,
    polyline_length, road_class_priority, road_declutter_invariant_holds,
    road_declutter_min_dist_m, road_name_schema_holds, road_name_visible_for_class,
    road_placement_geometry_holds, upright_angle_deg,
};
pub use roads::{
    CENTERLINE_DEDUPE_M, RoadSegment, extract_road_centerline, parse_roads_payload,
    road_style_width,
};
pub use store::{WorldError, WorldStore};
