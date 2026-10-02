//! Role: state.
//! Position: `camera/ortho` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::math::dimensions::or_one;
use crate::camera::math::dimensions::round_dimension;

/// Deck default near plane (`orthographic-viewport.js`: `near = 0.1`).
pub const NEAR: f64 = 0.1;

/// Deck default far plane (`orthographic-viewport.js`: `far = 1000`).
pub const FAR: f64 = 1000.0;

/// Zoom clamp floor — mirrors `useOrthographicView.ts` `MIN_ZOOM` (whole terrain visible).
pub const MIN_ZOOM: f64 = -6.0;

/// Zoom clamp ceiling — mirrors `useOrthographicView.ts` `MAX_ZOOM` (close inspection).
pub const MAX_ZOOM: f64 = 6.0;

/// The orthographic tactical-map camera. See the module docs for the parity contract.
#[derive(Clone, Debug)]
pub struct OrthoCamera {
    /// Width px.
    pub(crate) width_px: f64,

    /// Height px.
    pub(crate) height_px: f64,

    /// Zoom.
    pub(crate) zoom: f64,

    /// Target.
    pub(crate) target: [f64; 2],

    /// Scale.
    pub(crate) scale: f64,

    /// Bounds.
    pub(crate) bounds: Option<[f64; 4]>,
}

impl OrthoCamera {
    /// New camera; no clamping is applied to any argument (parity corpus requirement). Dimensions go through the same `Math.round` the app's `view.makeViewport` path applies.
    #[must_use]
    pub fn new(width_px: f64, height_px: f64, target_x: f64, target_y: f64, zoom: f64) -> Self {
        Self {
            width_px: round_dimension(width_px),
            height_px: round_dimension(height_px),
            zoom,
            target: [target_x, target_y],
            scale: zoom.exp2(),
            bounds: None,
        }
    }
}

impl OrthoCamera {
    /// Test-only oracle hook (plan gate T3): construct with an **injected** `scale` so the parity tests can split "pow drift" (T2) from "pipeline drift" (T3, asserted ULP == 0). Production code must never call this.
    #[doc(hidden)]
    #[must_use]
    pub fn with_scale_for_test(
        width_px: f64,
        height_px: f64,
        target_x: f64,
        target_y: f64,
        zoom: f64,
        scale: f64,
    ) -> Self {
        Self {
            width_px: round_dimension(width_px),
            height_px: round_dimension(height_px),
            zoom,
            target: [target_x, target_y],
            scale,
            bounds: None,
        }
    }
}

impl OrthoCamera {
    /// Camera target x (world meters).
    #[must_use]
    pub fn target_x(&self) -> f64 {
        self.target[0]
    }
}

impl OrthoCamera {
    /// Camera target y (world meters).
    #[must_use]
    pub fn target_y(&self) -> f64 {
        self.target[1]
    }
}

impl OrthoCamera {
    /// Current zoom (log2 pixels per meter).
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.zoom
    }
}

impl OrthoCamera {
    /// `2^zoom` — pixels per meter.
    #[must_use]
    pub fn scale(&self) -> f64 {
        self.scale
    }
}

impl OrthoCamera {
    /// Viewport size in CSS pixels after deck's `|| 1` coercion.
    #[must_use]
    pub fn size_px(&self) -> [f64; 2] {
        [or_one(self.width_px), or_one(self.height_px)]
    }
}

impl OrthoCamera {
    /// Set viewport dimensions (CSS pixels; `Math.round`ed like `makeViewport`).
    pub fn resize(&mut self, width_px: f64, height_px: f64) {
        self.width_px = round_dimension(width_px);
        self.height_px = round_dimension(height_px);
    }
}
