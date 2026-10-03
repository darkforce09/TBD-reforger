//! **Role:** viewport — where the camera is, and who hears that it moved: the engine's resize,
//! view, pan, zoom and camera-changed entry points.
//! **Position:** the map renderer; the Mission Creator's navigation, pointer gestures and wheel
//! zoom and the debug benches move the camera through these methods and then report the change.
//! **Signals & state:** the engine's camera, GPU context surface and damage flag; the engine's
//! frame hooks run on a camera change.
//! **Invariants:** every camera move marks damage; the surface is sized at `round(css·dpr)` device
//! pixels with JavaScript's rounding rule, the rounding the canvas backing size uses, and a
//! non-positive size or ratio is refused, never clamped.

use crate::engine::RenderEngine;
use crate::error::Result;
use crate::surface_size::device_surface_size;

impl RenderEngine {
    /// Resize: the camera in CSS pixels, the surface at `round(css·dpr)` device pixels (which
    /// must equal the canvas backing size the host just set, with the same rounding).
    ///
    /// # Errors
    /// [`crate::Error::NonPositiveResize`] when a side or the ratio is not positive; the camera
    /// and the surface are left as they were.
    pub fn resize(&mut self, css_w: f64, css_h: f64, dpr: f64) -> Result<()> {
        let (width, height) = device_surface_size(css_w, css_h, dpr)?;
        self.camera.resize(css_w, css_h);
        self.gpu.resize(width, height)?;
        self.damage.mark();
        Ok(())
    }
}

impl RenderEngine {
    /// Set the full view state (clamped like the editor's view-state layer).
    pub fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        self.camera.set_view(target_x, target_y, zoom);
        self.damage.mark();
    }
}

impl RenderEngine {
    /// Drag-pan by CSS-pixel deltas (content follows the cursor).
    pub fn pan(&mut self, dx_px: f64, dy_px: f64) {
        self.camera.pan(dx_px, dy_px);
        self.damage.mark();
    }
}

impl RenderEngine {
    /// Set camera bounds.
    pub fn set_camera_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.camera.set_bounds(min_x, min_y, max_x, max_y);
        self.damage.mark();
    }
}

impl RenderEngine {
    /// Cursor-anchored zoom (clamped to the view-state band).
    pub fn zoom_at(&mut self, dz: f64, cursor_x_px: f64, cursor_y_px: f64) {
        self.camera.zoom_at(dz, cursor_x_px, cursor_y_px);
        self.damage.mark();
    }
}

impl RenderEngine {
    /// Target x.
    #[must_use]
    pub fn target_x(&self) -> f64 {
        self.camera.target_x()
    }
}

impl RenderEngine {
    /// Target y.
    #[must_use]
    pub fn target_y(&self) -> f64 {
        self.camera.target_y()
    }
}

impl RenderEngine {
    /// Zoom.
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.camera.zoom()
    }
}

impl RenderEngine {
    /// Visible world rect `[minX, minY, maxX, maxY]` (deck `getBounds` parity — the future culling primitive).
    #[must_use]
    pub fn visible_bounds(&self) -> Vec<f64> {
        self.camera.visible_world_rect().to_vec()
    }
}

impl RenderEngine {
    /// Camera moved: every registered frame hook hears it, in registration order (the slot
    /// symbology re-derives its pixels-to-metres uniform, cluster gate and cluster markers). The
    /// hook list is out of the engine while the hooks run, because each hook is lent the whole
    /// engine.
    pub fn on_camera_changed(&mut self) {
        let mut hooks = std::mem::take(&mut self.frame_hooks);
        hooks.camera_changed(self);
        self.frame_hooks = hooks;
    }
}
