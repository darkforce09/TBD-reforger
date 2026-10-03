//! Where the elevation raster sits on the world.
//!
//! **Role:** [`DemManifest`], the raster's world rectangle, pixel size, axis flips and height
//! range as a terrain manifest's `dem` block declares them, and [`PixelCoord`], a continuous
//! position on the raster.
//! **Position:** built by the map engine's terrain boot and the developer tools from a terrain
//! manifest; read by [`crate::sampling`], the line of sight and the spot heights.
//! **Signals & state:** none; plain data.
//! **Invariants:** the rectangle spans `min` to `max` on both axes in world metres; a flip mirrors
//! its axis across the rectangle.

/// Raster dimensions and elevation encoding supplied to terrain samplers.
#[derive(Clone, Copy, Debug)]
pub struct DemManifest {
    /// West edge of the raster's world rectangle along X, in metres.
    pub min_x: f64,

    /// Near edge of the raster's world rectangle along world Z, in metres.
    pub min_y: f64,

    /// East edge of the raster's world rectangle along X, in metres (12 800 on Everon).
    pub max_x: f64,

    /// Far edge of the raster's world rectangle along world Z, in metres.
    pub max_y: f64,

    /// Raster width in pixels; `max_x` maps onto pixel column `width_px - 1`.
    pub width_px: usize,

    /// Raster height in pixels; `max_y` maps onto pixel row `height_px - 1`.
    pub height_px: usize,

    /// When set, pixel columns run from `max_x` back to `min_x` (`u = 1 - u`).
    pub flip_x: bool,

    /// When set, pixel rows run from `max_y` back to `min_y` along world Z (`v = 1 - v`).
    pub flip_z: bool,

    /// Height in metres a stored `u16` of 0 encodes (-204.78 on Everon).
    pub height_min_m: f64,

    /// Height in metres a stored `u16` of 65 535 encodes (375.53 on Everon).
    pub height_max_m: f64,
}

/// Continuous pixel coordinate on the heightmap (mirror of the `worldToPixel` return).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelCoord {
    /// Normalised position across the rectangle along X, 0 to 1 inside it, after any flip.
    pub u: f64,

    /// Normalised position across the rectangle along world Z, 0 to 1 inside it, after any
    /// flip.
    pub v: f64,

    /// Continuous pixel column, `u * (width_px - 1)`; inside the raster in `0..=width_px - 1`.
    pub px: f64,

    /// Continuous pixel row, `v * (height_px - 1)`; inside the raster in `0..=height_px - 1`.
    pub py: f64,
}
