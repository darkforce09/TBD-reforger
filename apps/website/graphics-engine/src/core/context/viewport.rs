//! Role: viewport.
//! Position: `core/context` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use wasm_bindgen::prelude::*;

/// Js round.
pub(crate) fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[wasm_bindgen]
impl RenderEngine {
    /// Resize: camera in CSS px; surface at `round(css·dpr)` device px (must equal the canvas backing size JS just set via `deviceSize` — same rounding function).
    pub fn resize(&mut self, css_w: f64, css_h: f64, dpr: f64) -> Result<(), JsError> {
        if !(css_w > 0.0 && css_h > 0.0 && dpr > 0.0) {
            return Err(JsError::new("resize-nonpositive"));
        }
        self.camera.resize(css_w, css_h);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            self.config.width = (js_round(css_w * dpr).max(1.0)) as u32;
            self.config.height = (js_round(css_h * dpr).max(1.0)) as u32;
        }
        self.surface.configure(&self.device, &self.config);
        self.damage.mark();
        Ok(())
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set the full view state (clamped like the editor's view-state layer).
    pub fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        self.camera.set_view(target_x, target_y, zoom);
        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Drag-pan by CSS-pixel deltas (content follows cursor). Live caller: the spike page's pointer pan (`WgpuCanvas.tsx`); the editor pans via `set_view` (audit X-05 corrected — this is not dead code).
    pub fn pan(&mut self, dx_px: f64, dy_px: f64) {
        self.camera.pan(dx_px, dy_px);
        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Set camera bounds.
    pub fn set_camera_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.camera.set_bounds(min_x, min_y, max_x, max_y);
        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Cursor-anchored zoom (clamped to the view-state band).
    pub fn zoom_at(&mut self, dz: f64, cursor_x_px: f64, cursor_y_px: f64) {
        self.camera.zoom_at(dz, cursor_x_px, cursor_y_px);
        self.damage.mark();
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Target x.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn target_x(&self) -> f64 {
        self.camera.target_x()
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Target y.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn target_y(&self) -> f64 {
        self.camera.target_y()
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Zoom.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.camera.zoom()
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Visible world rect `[minX, minY, maxX, maxY]` (deck `getBounds` parity — the future culling primitive).
    #[must_use]
    pub fn visible_bounds(&self) -> Vec<f64> {
        self.camera.visible_world_rect().to_vec()
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Camera moved: px_to_m + cluster gate re-eval (zoom is engine SoT).
    pub fn on_camera_changed(&mut self) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        self.sync_slot_zoom_uniform();
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        let cm = crate::symbology::instances::symbols::cluster_mode(n, zoom);
        let mode_changed = cm != self.slot_bridge.last_cluster_mode;
        if mode_changed {
            self.slot_bridge.last_cluster_mode = cm;
            if !self.slot_bridge.drag_active {
                self.rematerialize_slot_lane();
            }
        }

        self.feed_cluster_markers();
    }
}
