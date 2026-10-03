//! **Role:** the engine as the frame pump's `FrameTarget`: the two engine calls each animation
//! frame makes.
//! **Position:** the map renderer, over `gpu_frame::frame_pump`; the Mission Creator and the debug
//! benches start a `RafPump` on an `EngineHandle`, which calls these.
//! **Signals & state:** none of its own.
//! **Invariants:** the pump's render-then-poll order is the frame pump's; this file only says which
//! two engine methods those are, and a frame that fails never stops the loop.

use crate::engine::RenderEngine;
use gpu_frame::frame_pump::FrameTarget;

/// The engine as the renderer's frame loop sees it.
///
/// A trait impl on the renderer's own type, so the GPU frame crate that owns the trait and the
/// pump never learns the word `RenderEngine`.
impl FrameTarget for RenderEngine {
    fn render_frame(&mut self) {
        // The error is deliberately dropped: a frame that fails must not stop the loop. A host
        // that cannot afford the timer's readback mapping turns it off with
        // `disable_frame_timing`.
        let _ = self.render();
    }

    fn poll_device(&self) {
        // Drain the readback `map_async` callbacks so the next submit cannot double-map.
        self.poll();
    }
}
