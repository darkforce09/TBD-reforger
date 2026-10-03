//! The GPU frame's error messages: each is the stable tag logs and readouts match on.

use crate::error::Error;

#[test]
fn an_atlas_pixel_length_error_reads_as_its_atlas_tag() {
    let text = Error::AtlasPixelLength {
        atlas: "text-atlas",
        width: 16,
        height: 6,
        actual: 3,
    };
    assert_eq!(text.to_string(), "text-atlas-rgba-size");
    let glyph = Error::AtlasPixelLength {
        atlas: "glyph-atlas",
        width: 1,
        height: 1,
        actual: 0,
    };
    assert_eq!(glyph.to_string(), "glyph-atlas-rgba-size");
}
