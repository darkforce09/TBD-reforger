//! Role: pump.
//! Position: `renderers/engine` in the graphics engine.
//! Signals & state: none — this file is the seam between the engine and the renderer's frame
//! loop, and holds no state of its own.
//! Invariants: the pump's render→poll order is `website-graphics-engine`'s; this file only
//! says which two engine methods those are. The re-exports below are the app's only route to
//! the pump — the frontend depends on this crate, never on the renderer directly.

pub use website_graphics_engine::r#loop::FrameTarget;
pub use website_graphics_engine::r#loop::RafPump;

use crate::core::context::state::RenderEngine;

/// The engine as the renderer's frame loop sees it.
///
/// A trait impl, not an inherent one, and it lives HERE rather than in the graphics crate for
/// two independent reasons: inherent-impl coherence (E0116) would force all ~124 of
/// `RenderEngine`'s `impl` blocks across the wall with it, and `#[wasm_bindgen]` — which this
/// type's whole JS surface is built from — refuses trait impls outright, so the pump could
/// never have been a `#[wasm_bindgen]` method on it either. Implementing the renderer's trait
/// on our own type satisfies both, and keeps `website-graphics-engine` ignorant of the word
/// `RenderEngine`.
impl FrameTarget for RenderEngine {
    fn render_frame(&mut self) {
        // The error is deliberately dropped: a frame that fails must not stop the loop. The
        // timer double-map that panicked the 15.0 loop is handled upstream by
        // `disable_frame_timing`.
        let _ = self.render();
    }

    fn poll_device(&self) {
        // ★ T-159.15.1: drain readback `map_async` so the next submit can't double-map.
        self.poll();
    }
}
