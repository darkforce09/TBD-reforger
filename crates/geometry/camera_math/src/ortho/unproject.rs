//! The orthographic camera's pixel-to-world unprojection.
//!
//! **Role:** the inverted pixel projection, a pixel's world point on the plane z = 0, and the
//! visible world rectangle.
//! **Position:** methods of [`super::state::OrthoCamera`]; the controls' cursor-anchored zoom and
//! the map engine's picking call them.
//! **Signals & state:** none; reads the camera.
//! **Invariants:** a singular pixel matrix answers `[NaN, NaN]` rather than a guess; the visible
//! rectangle is the component-wise minimum and maximum of the four corners.

use crate::dimensions::js_max4;
use crate::dimensions::js_min4;
use crate::matrix4::invert;
use crate::matrix4::lerp2;
use crate::matrix4::transform_vector;
use crate::ortho::state::OrthoCamera;

impl OrthoCamera {
    /// `mat4.invert(pixelProjection)` — `None` iff singular (deck logs a warning and keeps `null`; unreachable for finite non-degenerate view states).
    #[must_use]
    pub fn pixel_unprojection(&self) -> Option<[f64; 16]> {
        invert(&self.pixel_projection())
    }
}

impl OrthoCamera {
    /// `viewport.unproject([px, py])` (no z, `topLeft: true`) — the two-point lerp onto the world z=0 plane (`pixelsToWorld` in `@math.gl/web-mercator`). Returns `[NaN, NaN]` if the pixel matrix is singular (deck would have warned at construction).
    #[must_use]
    pub fn unproject_xy(&self, px: f64, py: f64) -> [f64; 2] {
        let Some(m_inv) = self.pixel_unprojection() else {
            return [f64::NAN, f64::NAN];
        };
        let coord0 = transform_vector(&m_inv, [px, py, 0.0, 1.0]);
        let coord1 = transform_vector(&m_inv, [px, py, 1.0, 1.0]);
        let z0 = coord0[2];
        let z1 = coord1[2];

        let t = if z0 == z1 {
            0.0
        } else {
            (0.0 - z0) / (z1 - z0)
        };
        lerp2([coord0[0], coord0[1]], [coord1[0], coord1[1]], t)
    }
}

impl OrthoCamera {
    /// `viewport.getBounds()` → `[minX, minY, maxX, maxY]` — component-wise `Math.min`/`max` of the four viewport-corner unprojects at z=0 (`viewport.js` lines 188–200).
    #[must_use]
    pub fn visible_world_rect(&self) -> [f64; 4] {
        let [w, h] = self.size_px();
        let top_left = self.unproject_xy(0.0, 0.0);
        let top_right = self.unproject_xy(w, 0.0);
        let bottom_left = self.unproject_xy(0.0, h);
        let bottom_right = self.unproject_xy(w, h);
        [
            js_min4(top_left[0], top_right[0], bottom_left[0], bottom_right[0]),
            js_min4(top_left[1], top_right[1], bottom_left[1], bottom_right[1]),
            js_max4(top_left[0], top_right[0], bottom_left[0], bottom_right[0]),
            js_max4(top_left[1], top_right[1], bottom_left[1], bottom_right[1]),
        ]
    }
}
