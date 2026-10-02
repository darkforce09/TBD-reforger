//! Role: manifest.
//! Position: `world/terrain/dem` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Raster dimensions and elevation encoding supplied to terrain samplers.
#[derive(Clone, Copy, Debug)]
pub struct DemManifest {
    /// Min x.
    pub min_x: f64,

    /// Min y.
    pub min_y: f64,

    /// Max x.
    pub max_x: f64,

    /// Max y.
    pub max_y: f64,

    /// Width px.
    pub width_px: usize,

    /// Height px.
    pub height_px: usize,

    /// Flip x.
    pub flip_x: bool,

    /// Flip z.
    pub flip_z: bool,

    /// Height min m.
    pub height_min_m: f64,

    /// Height max m.
    pub height_max_m: f64,
}

/// Continuous pixel coordinate on the heightmap (mirror of the `worldToPixel` return).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelCoord {
    /// U.
    pub u: f64,

    /// V.
    pub v: f64,

    /// Px.
    pub px: f64,

    /// Py.
    pub py: f64,
}
