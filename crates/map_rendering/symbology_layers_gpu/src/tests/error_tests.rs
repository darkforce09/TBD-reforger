//! The error messages keep the stable atlas tags that logs and readouts match on.

use super::Error;

#[test]
fn glyph_atlas_uv_count_names_the_capacity_and_the_count() {
    let error = Error::GlyphAtlasUvCount {
        capacity: 1024,
        actual: 1028,
    };
    assert_eq!(
        error.to_string(),
        "glyph-atlas-uv-count: capacity 1024, got 1028"
    );
}

#[test]
fn slot_atlas_pixel_length_is_the_bare_tag() {
    let error = Error::SlotAtlasPixelLength {
        width: 4,
        height: 4,
        actual: 3,
    };
    assert_eq!(error.to_string(), "slot-atlas-rgba-size");
}

#[test]
fn glyph_atlas_build_forwards_the_gpu_frame_message() {
    let error = Error::from(gpu_frame::Error::AtlasPixelLength {
        atlas: "glyph-atlas",
        width: 2,
        height: 2,
        actual: 1,
    });
    assert_eq!(error.to_string(), "glyph-atlas-rgba-size");
}
