//! The road export image styles, layer files and draw order.
//!
//! **Role:** pins the behaviour of `export_image_styling.rs` named in each case.
//! **Position:** mounted by `export_image_styling.rs` under `cfg(test)`.
//! **Signals & state:** none.
//! **Invariants:** every class of `ROAD_CLASSES` is covered by each table exactly once.

use crate::export_image_styling::*;
use crate::road_class::ROAD_CLASSES;

#[test]
fn every_road_class_has_an_export_style() {
    for road_class in ROAD_CLASSES {
        let style = road_export_image_style(road_class);
        assert!(style.is_some(), "{road_class} has no export style");
    }
    assert_eq!(road_export_image_style("canal"), None);
    assert_eq!(road_export_image_style(""), None);
}

#[test]
fn the_layer_files_follow_the_class_table_and_round_trip() {
    let classes: Vec<&str> = ROAD_EXPORT_LAYER_FILES
        .iter()
        .map(|(_, road_class)| *road_class)
        .collect();
    assert_eq!(classes, ROAD_CLASSES);
    for (file_stem, road_class) in ROAD_EXPORT_LAYER_FILES {
        assert_eq!(road_export_layer_file_stem(road_class), Some(file_stem));
    }
    assert_eq!(road_export_layer_file_stem("highways"), None);
}

#[test]
fn the_draw_order_is_a_permutation_of_the_class_table() {
    let mut drawn = ROAD_EXPORT_DRAW_ORDER.to_vec();
    drawn.sort_unstable();
    let mut classes = ROAD_CLASSES.to_vec();
    classes.sort_unstable();
    assert_eq!(drawn, classes);
    assert_eq!(ROAD_EXPORT_DRAW_ORDER.first(), Some(&"runway"));
    assert_eq!(ROAD_EXPORT_DRAW_ORDER.last(), Some(&"highway_paved"));
}
