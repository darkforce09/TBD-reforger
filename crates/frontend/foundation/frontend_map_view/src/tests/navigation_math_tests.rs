//! Wheel zoom rate, click-versus-drag, and pixel-to-metre conversion.

use super::super::camera_fit::ViewState;
use super::{CLICK_SLOP_PX, WHEEL_ZOOM_PER_PX, is_click, map_metres_at, wheel_zoom_delta};

#[test]
fn wheel_down_zooms_out_one_level_per_five_hundred_pixels() {
    assert_eq!(WHEEL_ZOOM_PER_PX, 1.0 / 500.0);
    assert!((wheel_zoom_delta(500.0) - (-1.0)).abs() < 1e-12);
    assert!((wheel_zoom_delta(-250.0) - 0.5).abs() < 1e-12);
}

#[test]
fn a_release_within_the_slop_is_a_click() {
    assert!(is_click((10.0, 10.0), (10.0, 10.0)));
    assert!(is_click((10.0, 10.0), (10.0 + CLICK_SLOP_PX, 10.0)));
    assert!(!is_click((10.0, 10.0), (13.0, 13.0)));
    assert!(!is_click((0.0, 0.0), (0.0, -CLICK_SLOP_PX - 0.01)));
}

#[test]
fn the_container_centre_is_the_camera_target() {
    let view = ViewState {
        target_x: 6_400.0,
        target_y: 3_200.0,
        zoom: -2.0,
    };
    let (x, y) = map_metres_at(800.0, 600.0, view, 400.0, 300.0).expect("finite");
    assert!(
        (x - 6_400.0).abs() < 1e-6 && (y - 3_200.0).abs() < 1e-6,
        "({x}, {y})"
    );
}

#[test]
fn pixels_scale_by_the_zoom_with_north_up() {
    // zoom 1: two CSS pixels per metre.
    let view = ViewState {
        target_x: 100.0,
        target_y: 100.0,
        zoom: 1.0,
    };
    let (x, y) = map_metres_at(200.0, 200.0, view, 200.0, 0.0).expect("finite");
    assert!((x - 150.0).abs() < 1e-6, "right edge is east: {x}");
    assert!((y - 150.0).abs() < 1e-6, "top edge is north: {y}");
    let (x, y) = map_metres_at(200.0, 200.0, view, 0.0, 200.0).expect("finite");
    assert!(
        (x - 50.0).abs() < 1e-6 && (y - 50.0).abs() < 1e-6,
        "({x}, {y})"
    );
}

#[test]
fn an_empty_container_has_no_map_position() {
    let view = ViewState {
        target_x: 0.0,
        target_y: 0.0,
        zoom: 0.0,
    };
    assert_eq!(map_metres_at(0.0, 100.0, view, 0.0, 0.0), None);
}
