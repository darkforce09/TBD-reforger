//! Role: controllers.
//! Position: `camera/ortho` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::math::dimensions::clamp;
use crate::camera::ortho::state::MAX_ZOOM;
use crate::camera::ortho::state::MIN_ZOOM;
use crate::camera::ortho::state::OrthoCamera;

impl OrthoCamera {
    /// Set the target clamp rect (the spike page mirrors the editor: `[0, 0, 12800, 12800]`).
    pub fn set_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.bounds = Some([min_x, min_y, max_x, max_y]);
    }
}

impl OrthoCamera {
    /// Set the full view state, applying the view-state-layer clamps exactly as `onViewStateChange` does: zoom to [`MIN_ZOOM`], [`MAX_ZOOM`]; target to `bounds` if set.
    pub fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        self.zoom = clamp(zoom, MIN_ZOOM, MAX_ZOOM);
        self.scale = self.zoom.exp2();
        self.target = [target_x, target_y];
        self.clamp_target();
    }
}

impl OrthoCamera {
    /// Clamp target.
    pub(crate) fn clamp_target(&mut self) {
        if let Some([min_x, min_y, max_x, max_y]) = self.bounds {
            self.target[0] = clamp(self.target[0], min_x, max_x);
            self.target[1] = clamp(self.target[1], min_y, max_y);
        }
    }
}

impl OrthoCamera {
    /// Drag-pan by a screen-pixel delta — content follows the cursor. Screen +x ⇒ target west (`-= dx/scale`); screen +y (down) ⇒ target north (`+= dy/scale`, flipY:false).
    pub fn pan(&mut self, dx_px: f64, dy_px: f64) {
        self.target[0] -= dx_px / self.scale;
        self.target[1] += dy_px / self.scale;
        self.clamp_target();
    }
}

impl OrthoCamera {
    /// Zoom by `dz` (clamped to [[`MIN_ZOOM`], [`MAX_ZOOM`]]) keeping the world point under the cursor `(cx, cy)` (top-left CSS px) fixed on screen.
    pub fn zoom_at(&mut self, dz: f64, cursor_x_px: f64, cursor_y_px: f64) {
        let world = self.unproject_xy(cursor_x_px, cursor_y_px);
        self.zoom = clamp(self.zoom + dz, MIN_ZOOM, MAX_ZOOM);
        self.scale = self.zoom.exp2();
        let [w, h] = self.size_px();
        self.target[0] = world[0] - (cursor_x_px - w / 2.0) / self.scale;
        self.target[1] = world[1] + (cursor_y_px - h / 2.0) / self.scale;
        self.clamp_target();
    }
}
