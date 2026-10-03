//! The damage-driven frame pump of a map view.
//!
//! **Role:** starts the shared `requestAnimationFrame` pump over a map view's engine, with an
//! optional per-frame hook for readouts that follow the camera.
//! **Position:** wraps [`map_engine::frame::RafPump`]; the Mission Creator's frame loop
//! (`apps/editor/bridge/viewport.rs`) and [`super::mount::mount_map_view`] start it.
//! **Signals & state:** the pump owns its frame counter; the hook owns whatever it captures.
//! **Invariants:** the engine draws only when damaged (camera, resize or content change); the pump
//! stops at the first frame after `disposed` is set.

use map_engine::frame::engine::RenderEngine;
use map_engine::frame::{EngineHandle, RafPump};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Start the pump with `after_frame(engine, frames_drawn)` run after every drawn frame.
pub fn start_frame_pump(
    engine: EngineHandle,
    disposed: Arc<AtomicBool>,
    after_frame: impl FnMut(&mut RenderEngine, u32) + 'static,
) {
    RafPump::new(engine, disposed)
        .after_frame(after_frame)
        .start();
}

/// Start the pump with no per-frame hook.
pub fn start_plain_frame_pump(engine: EngineHandle, disposed: Arc<AtomicBool>) {
    RafPump::new(engine, disposed).start();
}
