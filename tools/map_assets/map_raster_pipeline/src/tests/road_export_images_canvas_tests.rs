//! Unit tests of the road export canvas.
//!
//! **Role:** pins [`super::RoadCanvas`]'s "over" blend, its feathered disc stamp and its two row
//! exports to hand-computed values.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by
//! `road_export_images/road_canvas.rs`.
//! **Signals & state:** none; each case builds its own small canvas.
//! **Invariants:** every expected value is the rounded result of the documented formula in `f64`.

use super::RoadCanvas;

const AMBER: [u8; 3] = [255, 175, 45];
const BACKGROUND: [u8; 3] = [18, 22, 28];

#[test]
fn opaque_blend_replaces_whatever_the_pixel_held() {
    let mut canvas = RoadCanvas::new(3, 2);
    canvas.blend_pixel(1, 1, [10, 20, 30], 90);
    canvas.blend_pixel(1, 1, AMBER, 255);
    assert_eq!(canvas.pixel(1, 1), [255, 175, 45, 255]);
    canvas.blend_pixel(1, 1, [1, 2, 3], 255);
    assert_eq!(canvas.pixel(1, 1), [1, 2, 3, 255]);
}

#[test]
fn translucent_blend_over_transparent_keeps_the_colour_and_alpha() {
    let mut canvas = RoadCanvas::new(2, 2);
    canvas.blend_pixel(0, 1, AMBER, 220);
    assert_eq!(canvas.pixel(0, 1), [255, 175, 45, 220]);
    assert_eq!(canvas.pixel(1, 1), [0, 0, 0, 0]);
}

#[test]
fn half_alpha_over_half_alpha_weights_both_colours() {
    let mut canvas = RoadCanvas::new(1, 1);
    canvas.blend_pixel(0, 0, [200, 100, 50], 128);
    assert_eq!(canvas.pixel(0, 0), [200, 100, 50, 128]);
    // o = 128/255 + 128/255 × 127/255 = 0.75196…, alpha round(191.749) = 192; each channel is
    // (0 + p × 128/255 × 127/255) / o: 66.49, 33.25, 16.62.
    canvas.blend_pixel(0, 0, [0, 0, 0], 128);
    assert_eq!(canvas.pixel(0, 0), [66, 33, 17, 192]);
}

#[test]
fn zero_alpha_over_transparent_leaves_the_pixel() {
    let mut canvas = RoadCanvas::new(1, 1);
    canvas.blend_pixel(0, 0, AMBER, 0);
    assert_eq!(canvas.pixel(0, 0), [0, 0, 0, 0]);
}

#[test]
fn blend_outside_the_canvas_is_ignored() {
    let mut canvas = RoadCanvas::new(2, 2);
    canvas.blend_pixel(2, 0, AMBER, 255);
    canvas.blend_pixel(0, 2, AMBER, 255);
    for y in 0..2 {
        for x in 0..2 {
            assert_eq!(canvas.pixel(x, y), [0, 0, 0, 0]);
        }
    }
}

#[test]
fn disc_edge_is_feathered_over_the_last_three_quarters_of_a_pixel() {
    let mut canvas = RoadCanvas::new(11, 11);
    canvas.draw_disc(5.0, 5.0, 2.0, AMBER, 255);
    // The centre and its direct neighbours sit at least 0.75 px inside the edge.
    assert_eq!(canvas.pixel(5, 5), [255, 175, 45, 255]);
    assert_eq!(canvas.pixel(6, 5), [255, 175, 45, 255]);
    // A diagonal neighbour is 2 − √2 = 0.586 px inside: round(255 × 0.586 / 0.75) = 199.
    assert_eq!(canvas.pixel(6, 6), [255, 175, 45, 199]);
    // A pixel exactly on the rim gets coverage 0 and is never blended.
    assert_eq!(canvas.pixel(7, 5), [0, 0, 0, 0]);
    assert_eq!(canvas.pixel(8, 5), [0, 0, 0, 0]);
}

#[test]
fn nan_disc_and_nan_segment_draw_nothing() {
    let mut canvas = RoadCanvas::new(4, 4);
    canvas.draw_disc(f64::NAN, 1.0, 2.0, AMBER, 255);
    canvas.draw_segment([0.0, 0.0], [f64::NAN, 3.0], 2.0, AMBER, 255);
    canvas.draw_segment([0.0, 0.0], [3.0, 3.0], f64::NAN, AMBER, 255);
    let mut row = vec![1_u8; 16];
    for row_index in 0..4 {
        canvas.fill_rgba_row(row_index, &mut row);
        assert!(row.iter().all(|&byte| byte == 0));
    }
}

#[test]
fn rgba_row_copies_the_pixels() {
    let mut canvas = RoadCanvas::new(2, 2);
    canvas.blend_pixel(1, 1, AMBER, 220);
    let mut row = vec![9_u8; 8];
    canvas.fill_rgba_row(1, &mut row);
    assert_eq!(row, [0, 0, 0, 0, 255, 175, 45, 220]);
}

#[test]
fn rgb_row_blends_over_the_background_by_alpha() {
    let mut canvas = RoadCanvas::new(3, 1);
    canvas.blend_pixel(1, 0, [200, 100, 50], 255);
    canvas.blend_pixel(2, 0, [200, 100, 50], 128);
    let mut row = vec![0_u8; 9];
    canvas.fill_rgb_row_over(0, &mut row, BACKGROUND);
    // Alpha 0 is the background, 255 the pixel, 128 round(c × 128/255 + b × 127/255).
    assert_eq!(row, [18, 22, 28, 200, 100, 50, 109, 61, 39]);
}
