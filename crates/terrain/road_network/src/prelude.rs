//! The names a reader of the road network imports with `use road_network::prelude::*;`.

pub use crate::airfield::{
    AIRFIELD_BBOX_MARGIN_M, apron_qualifying_area_m2, build_airfield_apron_mesh,
    compute_airfield_bbox, is_airfield_structure_class, point_in_bbox,
};
pub use crate::cartographic_strip::{
    compose_bridge_rail_strips, compose_fence_strip, compose_pier_strip, pack_cartographic_strips,
};
pub use crate::error::{Error, Result};
pub use crate::export_image_styling::{
    ROAD_EXPORT_DARK_BACKGROUND_RGB, ROAD_EXPORT_DRAW_ORDER, ROAD_EXPORT_LAYER_FILES,
    ROAD_JUNCTION_ALPHA, ROAD_JUNCTION_MIN_DEGREE, ROAD_JUNCTION_RADIUS_PX, ROAD_JUNCTION_RGB,
    RoadExportImageStyle, road_export_image_style, road_export_layer_file_stem,
};
pub use crate::mesh::{RoadInput, RoadMeshGpu, compose_roads_mesh};
pub use crate::network::{
    RoadSegment, extract_road_centerline, parse_roads_payload, road_network_from_bytes,
};
pub use crate::road_class::{ROAD_CLASSES, road_class_code, road_class_name};
pub use crate::styling::{
    RoadStyle, StripVertex, expand_polyline_strip, road_class_signature, road_class_visible,
    road_style,
};
