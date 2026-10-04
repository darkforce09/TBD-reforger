//! The canvas backing-store size for a CSS box at a device pixel ratio.
//!
//! **Role:** turns the CSS size of a map container and the browser's device pixel ratio into the
//! device-pixel width and height the canvas element must carry before the render engine reads it.
//! **Position:** called by the map view's canvas sizing and resize handling
//! (`engine_mount`, `resize`), which the Mission Creator and every other map
//! view share.
//! **Signals & state:** none; pure functions.
//! **Invariants:** rounds half up exactly as the render engine's own `resize` does, so the canvas
//! and the configured surface always agree; never answers a zero dimension.

/// Device-pixel canvas size for a CSS box `css_w × css_h` at `dpr`: `floor(v · dpr + 0.5)`,
/// at least one pixel on each axis.
#[must_use]
pub fn device_size(css_w: f64, css_h: f64, dpr: f64) -> (u32, u32) {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let round = |v: f64| ((v * dpr + 0.5).floor().max(1.0)) as u32;
    (round(css_w), round(css_h))
}

#[cfg(test)]
#[path = "tests/device_size_tests.rs"]
mod tests;
