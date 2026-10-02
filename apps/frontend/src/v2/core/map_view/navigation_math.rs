//! The arithmetic behind map navigation: wheel zoom steps, the click-versus-drag test, and the
//! container pixel to map metre conversion.
//!
//! **Role:** the pure half of [`super::navigation`], kept apart so the native test build covers
//! it.
//! **Position:** called by the map view's pointer and wheel listeners; the Mission Creator's own
//! gestures use the same wheel rate.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a pixel converts to metres through the same orthographic camera the render
//! engine draws with, so a click lands on the ground under the pointer; a release within
//! [`CLICK_SLOP_PX`] of its press is a click, anything farther is a drag.

use super::camera_fit::ViewState;
use camera_math::ortho::state::OrthoCamera;

/// Zoom levels per CSS pixel of wheel travel (500 px of wheel is one zoom level, a factor of 2).
pub const WHEEL_ZOOM_PER_PX: f64 = 1.0 / 500.0;

/// Largest pointer travel, in CSS pixels, between press and release that still counts as a
/// click.
pub const CLICK_SLOP_PX: f64 = 4.0;

/// Zoom change for a wheel event's vertical delta; wheel down (positive delta) zooms out.
#[must_use]
pub fn wheel_zoom_delta(delta_y_px: f64) -> f64 {
    -delta_y_px * WHEEL_ZOOM_PER_PX
}

/// Whether a press at `down` released at `up` (both CSS pixels) is a click rather than a drag.
#[must_use]
pub fn is_click(down: (f64, f64), up: (f64, f64)) -> bool {
    (up.0 - down.0).hypot(up.1 - down.1) <= CLICK_SLOP_PX
}

/// Map metres under container pixel `(px, py)` (CSS pixels from the container's top-left) for a
/// `css_w × css_h` container showing `view`; `None` for an empty container or a non-finite
/// result.
#[must_use]
pub fn map_metres_at(
    css_w: f64,
    css_h: f64,
    view: ViewState,
    px: f64,
    py: f64,
) -> Option<(f64, f64)> {
    if !(css_w > 0.0 && css_h > 0.0) {
        return None;
    }
    let camera = OrthoCamera::new(css_w, css_h, view.target_x, view.target_y, view.zoom);
    let [x, y] = camera.unproject_xy(px, py);
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

#[cfg(test)]
#[path = "tests/navigation_math_tests.rs"]
mod tests;
