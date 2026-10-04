//! The world rectangle a terrain covers, and the camera view that fits it into a container.
//!
//! **Role:** reads a terrain manifest's `worldBounds` and derives the initial camera (target and
//! zoom) that shows the whole terrain in a container of a given CSS size.
//! **Position:** feeds `engine_mount::create_engine` with its camera bounds and first
//! view; a map view fetches `/map-assets/<terrain>/manifest.json` and hands the bytes here.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a bounds value is finite with a positive extent on both axes; a fitted zoom
//! stays inside the engine camera's `[MIN_ZOOM, MAX_ZOOM]` band, so the engine never clamps the
//! view the caller asked for.

use camera_math::ortho::state::{MAX_ZOOM, MIN_ZOOM};

/// A terrain's world rectangle in map metres (x east, y north).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBounds {
    /// West edge.
    pub min_x: f64,

    /// South edge.
    pub min_y: f64,

    /// East edge.
    pub max_x: f64,

    /// North edge.
    pub max_y: f64,
}

impl WorldBounds {
    /// Bounds from edges; `None` unless every edge is finite and both extents are positive.
    #[must_use]
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Option<Self> {
        let finite = [min_x, min_y, max_x, max_y].iter().all(|v| v.is_finite());
        (finite && max_x > min_x && max_y > min_y).then_some(Self {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }

    /// Bounds from a terrain manifest document's `worldBounds` array `[min_x, min_y, max_x,
    /// max_y]`; `None` when the document or the array is malformed.
    #[must_use]
    pub fn from_manifest_json(bytes: &[u8]) -> Option<Self> {
        #[derive(serde::Deserialize)]
        struct ManifestWorldBounds {
            #[serde(rename = "worldBounds")]
            world_bounds: [f64; 4],
        }
        let parsed: ManifestWorldBounds = serde_json::from_slice(bytes).ok()?;
        let [min_x, min_y, max_x, max_y] = parsed.world_bounds;
        Self::new(min_x, min_y, max_x, max_y)
    }

    /// East-west extent in metres.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// North-south extent in metres.
    #[must_use]
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// Centre point.
    #[must_use]
    pub fn centre(&self) -> (f64, f64) {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }
}

/// A camera view: the world point at the container centre and the zoom (log2 CSS pixels per
/// metre).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewState {
    /// Target x in metres.
    pub target_x: f64,

    /// Target y in metres.
    pub target_y: f64,

    /// Zoom, log2 of CSS pixels per metre.
    pub zoom: f64,
}

/// The view that centres `bounds` and shows all of it in a `css_w × css_h` container; a
/// container with no area gets the widest zoom.
#[must_use]
pub fn fit_view(bounds: WorldBounds, css_w: f64, css_h: f64) -> ViewState {
    let (target_x, target_y) = bounds.centre();
    let zoom = if css_w > 0.0 && css_h > 0.0 {
        let pixels_per_metre = (css_w / bounds.width()).min(css_h / bounds.height());
        pixels_per_metre.log2().clamp(MIN_ZOOM, MAX_ZOOM)
    } else {
        MIN_ZOOM
    };
    ViewState {
        target_x,
        target_y,
        zoom,
    }
}

#[cfg(test)]
#[path = "tests/camera_fit_tests.rs"]
mod tests;
