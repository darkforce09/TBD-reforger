//! The render engine's error messages keep the stable codes logs and readouts match on: its own
//! `resize-nonpositive`, and the code of every error it carries, unchanged.

use super::Error;

#[test]
fn a_non_positive_resize_names_its_code_and_the_requested_size() {
    let error = Error::NonPositiveResize {
        css_width: 0.0,
        css_height: 480.0,
        device_pixel_ratio: 2.0,
    };
    assert_eq!(
        error.to_string(),
        "resize-nonpositive: 0x480 css px at 2 dpr"
    );
}

#[test]
fn a_gpu_device_error_keeps_its_code() {
    let error = Error::from(gpu_device::Error::CanvasZeroSize {
        width: 0,
        height: 600,
    });
    assert!(
        error.to_string().starts_with("canvas-zero-size: "),
        "{error}"
    );
    assert_eq!(
        Error::from(gpu_device::Error::NoAdapter("none".to_owned())).to_string(),
        "no-adapter: none"
    );
}

#[test]
fn a_text_atlas_error_keeps_its_code() {
    let error = Error::from(gpu_frame::Error::AtlasPixelLength {
        atlas: "text-atlas",
        width: 4,
        height: 4,
        actual: 3,
    });
    assert_eq!(error.to_string(), "text-atlas-rgba-size");
}

#[test]
fn a_typed_layer_error_keeps_its_code() {
    let symbology = Error::from(symbology_layers_gpu::Error::GlyphAtlasUvCount {
        capacity: 8,
        actual: 9,
    });
    assert_eq!(
        symbology.to_string(),
        "glyph-atlas-uv-count: capacity 8, got 9"
    );
    let world = Error::from(world_layers_gpu::Error::TextureRole { role: 7 });
    assert_eq!(
        world.to_string(),
        "tex_layer: role must be 0 (basemap) or 1 (hillshade)"
    );
}
