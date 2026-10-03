//! The names a reader of the place names imports with `use place_names::prelude::*;`.

pub use crate::error::{Error, Result};
pub use crate::label_packing::{
    pack_height_label_glyphs, pack_road_label_bytes, pack_town_label_bytes, pack_town_label_glyphs,
};
pub use crate::peaks::{
    HeightLabel, HeightLabelKind, declutter_height_labels, find_peaks, height_labels_to_specs,
};
pub use crate::road_name_ids::RoadNameId;
pub use crate::route_labels::{
    RoadNameEntry, RoadNamesFile, build_road_label_draw_set_from_archive, parse_road_names_json,
    road_names_from_archive, road_names_to_archive,
};
pub use crate::route_placement::{
    RoadLabelPlacement, build_road_label_draw_set, declutter_road_labels, place_road_labels,
};
pub use crate::towns::{
    MapLabels, locations_to_label_specs, map_labels_from_bytes, parse_height_labels_json,
    parse_locations_json,
};
