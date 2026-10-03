//! Keeps the map steady while the Mission Creator's chrome and docks open and close.
//!
//! **Role:** mirrors the hide-chrome and dock-collapse signals into the shell layout latches,
//! re-sizes the engine to the container, and, when a dock reflow moves the map pane's centre,
//! shifts the camera target so the world point at the pane centre stays put.
//! **Position:** installed by the canvas mount; reads `crate::workspaces::editor::session::layout`
//! for the pane geometry.
//! **Signals & state:** one effect over `chrome_hidden`, `dock_left_collapsed` and
//! `dock_right_collapsed`; the layout latches it writes are the shell's.
//! **Invariants:** the latches are always written, even while the container has no size; the
//! centre hold applies only to a dock reflow with the chrome visible before and after, never to a
//! hide-chrome toggle.

use leptos::prelude::*;
use map_engine::frame::EngineHandle;

/// Install the chrome and dock reflow effect for the mounted engine and container.
pub(super) fn install(
    engine: EngineHandle,
    container: web_sys::HtmlDivElement,
    chrome_hidden: RwSignal<bool>,
    dock_left_collapsed: RwSignal<bool>,
    dock_right_collapsed: RwSignal<bool>,
) {
    use crate::workspaces::editor::session::layout;
    Effect::new(move |_| {
        let hidden = chrome_hidden.get();
        let left = dock_left_collapsed.get();
        let right = dock_right_collapsed.get();
        let was_hidden = layout::chrome_hidden();

        let rect = container.get_bounding_client_rect();
        let (w, h) = (rect.width(), rect.height());
        if !(w > 0.0 && h > 0.0) {
            layout::set_chrome_hidden(hidden);
            layout::set_dock_left_collapsed(left);
            layout::set_dock_right_collapsed(right);
            return;
        }

        let before = layout::pane_center_px(w, h);
        layout::set_chrome_hidden(hidden);
        layout::set_dock_left_collapsed(left);
        layout::set_dock_right_collapsed(right);
        let after = layout::pane_center_px(w, h);

        let dpr = web_sys::window().map_or(1.0, |win| win.device_pixel_ratio());
        if let Some(e) = engine.borrow_mut().as_mut() {
            let _ = e.resize(w, h, dpr);
            let dock_reflow = !was_hidden && !hidden;
            if dock_reflow
                && ((before.0 - after.0).abs() > f64::EPSILON
                    || (before.1 - after.1).abs() > f64::EPSILON)
            {
                let scale = e.zoom().exp2();
                let (nx, ny) =
                    layout::centre_hold_target(e.target_x(), e.target_y(), scale, before, after);
                e.set_view(nx, ny, e.zoom());
            }
        }
    });
}
