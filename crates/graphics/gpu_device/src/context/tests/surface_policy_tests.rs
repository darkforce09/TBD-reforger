//! The surface policy of the GPU bootstrap: size checks, format pick, backend kind, timestamps.

use crate::context::surface_policy::{
    BackendKind, checked_canvas_size, checked_surface_size, first_linear_format, request_timestamps,
};
use crate::error::Error;

#[test]
fn a_canvas_without_a_backing_size_is_refused() {
    assert_eq!(
        checked_canvas_size(0, 600),
        Err(Error::CanvasZeroSize {
            width: 0,
            height: 600
        })
    );
    assert_eq!(
        checked_canvas_size(800, 0),
        Err(Error::CanvasZeroSize {
            width: 800,
            height: 0
        })
    );
    assert_eq!(checked_canvas_size(800, 600), Ok((800, 600)));
}

#[test]
fn a_resize_to_a_zero_side_is_an_error_and_never_clamped() {
    assert_eq!(
        checked_surface_size(0, 0),
        Err(Error::NonPositiveSurfaceSize {
            width: 0,
            height: 0
        })
    );
    assert_eq!(checked_surface_size(1, 1), Ok((1, 1)));
    assert_eq!(checked_surface_size(3840, 2160), Ok((3840, 2160)));
}

#[test]
fn the_first_non_srgb_format_is_picked() {
    // (format id, is sRGB)
    let formats = [(1, true), (2, false), (3, false)];
    assert_eq!(first_linear_format(&formats, |f| f.1), Ok((2, false)));
}

#[test]
fn an_srgb_only_surface_is_refused() {
    let formats = [(1, true), (2, true)];
    assert_eq!(
        first_linear_format(&formats, |f| f.1),
        Err(Error::SrgbOnlySurface)
    );
    let none: [(u8, bool); 0] = [];
    assert_eq!(
        first_linear_format(&none, |f| f.1),
        Err(Error::SrgbOnlySurface)
    );
}

#[test]
fn the_backend_kind_follows_the_adapter_backend() {
    assert_eq!(BackendKind::from_is_gl(true), BackendKind::WebGl2);
    assert_eq!(BackendKind::from_is_gl(false), BackendKind::WebGpu);
    assert!(BackendKind::WebGl2.is_gl());
    assert!(!BackendKind::WebGpu.is_gl());
    assert_eq!(BackendKind::WebGpu.as_str(), "webgpu");
    assert_eq!(BackendKind::WebGl2.as_str(), "webgl2");
}

#[test]
fn timestamps_need_both_the_wish_and_the_adapter() {
    assert!(request_timestamps(true, true));
    assert!(!request_timestamps(true, false));
    assert!(!request_timestamps(false, true));
    assert!(!request_timestamps(false, false));
}

#[test]
fn error_messages_start_with_their_stable_code() {
    let cases = [
        (
            Error::CanvasZeroSize {
                width: 0,
                height: 1,
            },
            "canvas-zero-size: ",
        ),
        (Error::CreateSurface("x".into()), "create-surface: x"),
        (Error::NoAdapter("x".into()), "no-adapter: x"),
        (Error::NoDevice("x".into()), "no-device: x"),
        (Error::SrgbOnlySurface, "srgb-only-surface: "),
        (
            Error::SurfaceUnsupportedByAdapter,
            "surface-unsupported-by-adapter",
        ),
        (
            Error::NonPositiveSurfaceSize {
                width: 0,
                height: 2,
            },
            "surface-size-nonpositive: 0x2",
        ),
        (
            Error::SurfaceAcquire("Lost".into()),
            "surface-acquire: Lost",
        ),
        (
            Error::SurfaceAcquireAfterReconfigure("Lost".into()),
            "surface-acquire-after-reconfigure: Lost",
        ),
    ];
    for (error, prefix) in cases {
        assert!(
            error.to_string().starts_with(prefix),
            "{error} must start with {prefix}"
        );
    }
}
