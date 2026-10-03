//! The error messages keep the stable call tags and reason texts that logs and readouts match on.

use super::{Error, LayerCall};

#[test]
fn every_layer_call_has_its_stable_tag() {
    let tags = [
        (LayerCall::ForestDensityUpload, "forest_density_upload"),
        (LayerCall::ViewshedUpload, "viewshed_upload"),
        (LayerCall::TextureBegin, "tex_layer"),
        (LayerCall::TextureWriteBitmap, "tex_layer_write_bitmap"),
        (LayerCall::TextureWriteRgba, "tex_layer_write_rgba"),
        (LayerCall::TextureCommit, "tex_layer_commit"),
    ];
    for (call, tag) in tags {
        assert_eq!(call.tag(), tag);
        assert_eq!(call.to_string(), tag);
    }
}

#[test]
fn raster_refusals_start_with_the_call_tag() {
    let call = LayerCall::ForestDensityUpload;
    assert_eq!(
        Error::ZeroTextureDimensions { call }.to_string(),
        "forest_density_upload: zero texture dimensions"
    );
    assert_eq!(
        Error::RasterRowPitch {
            call: LayerCall::ViewshedUpload,
            bytes_per_row: 100,
            width: 64,
        }
        .to_string(),
        "viewshed_upload: bytes_per_row must be ≥ tex_w*4 and 256-aligned"
    );
    assert_eq!(
        Error::RasterSizeOverflow { call }.to_string(),
        "forest_density_upload: size overflow"
    );
    assert_eq!(
        Error::RasterByteLength {
            call: LayerCall::ViewshedUpload,
            expected: 512,
            actual: 511,
        }
        .to_string(),
        "viewshed_upload: rgba length mismatch"
    );
}

#[test]
fn texture_refusals_keep_their_reason_texts() {
    assert_eq!(
        Error::ZeroTextureDimensions {
            call: LayerCall::TextureBegin,
        }
        .to_string(),
        "tex_layer: zero texture dimensions"
    );
    assert_eq!(
        Error::TextureRole { role: 2 }.to_string(),
        "tex_layer: role must be 0 (basemap) or 1 (hillshade)"
    );
    assert_eq!(
        Error::TileByteLength {
            expected: 16,
            actual: 15,
        }
        .to_string(),
        "tex_layer_write_rgba: byte length != w*h*4"
    );
    for (call, message) in [
        (
            LayerCall::TextureWriteBitmap,
            "tex_layer_write_bitmap: begin not called",
        ),
        (
            LayerCall::TextureWriteRgba,
            "tex_layer_write_rgba: begin not called",
        ),
        (
            LayerCall::TextureCommit,
            "tex_layer_commit: begin not called",
        ),
    ] {
        assert_eq!(
            Error::TextureNotBegun { call, role: 0 }.to_string(),
            message
        );
    }
}
