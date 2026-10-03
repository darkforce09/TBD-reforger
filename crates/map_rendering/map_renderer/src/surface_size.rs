//! The map canvas's surface size policy.
//!
//! **Role:** [`device_surface_size`], the device-pixel size the render engine configures its surface
//! at for a CSS size and a device-pixel ratio.
//! **Position:** called by the engine's `resize` before it resizes the GPU context; the canvas
//! backing size the Mission Creator sets uses the same rounding.
//! **Signals & state:** none; a pure function.
//! **Invariants:** a non-positive (or NaN) CSS side or ratio is refused, never clamped; each side
//! is `round(css × dpr)` with JavaScript's rounding rule, and at least one pixel.

use crate::error::{Error, Result};
use map_coordinates::rounding::round;

/// The surface size in device pixels, `(width, height)`, for a canvas of `css_width` ×
/// `css_height` CSS pixels at `device_pixel_ratio`.
///
/// # Errors
/// [`Error::NonPositiveResize`] when a side or the ratio is not a positive number.
pub(crate) fn device_surface_size(
    css_width: f64,
    css_height: f64,
    device_pixel_ratio: f64,
) -> Result<(u32, u32)> {
    if !(css_width > 0.0 && css_height > 0.0 && device_pixel_ratio > 0.0) {
        return Err(Error::NonPositiveResize {
            css_width,
            css_height,
            device_pixel_ratio,
        });
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = |css: f64| round(css * device_pixel_ratio).max(1.0) as u32;
    Ok((side(css_width), side(css_height)))
}

#[cfg(test)]
#[path = "tests/surface_size_tests.rs"]
mod tests;
