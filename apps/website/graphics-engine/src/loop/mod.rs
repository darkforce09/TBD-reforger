//! Role: Module boundary for loop.
//! Position: `apps/website/graphics-engine/src` — the frame pump.
//! Signals & state: the rAF callback slot, the disposal flag, the frame counter.
//! Invariants: the pump never names a renderer. It drives a [`pump::FrameTarget`], so the
//! engine type stays in `website-map-engine` where its `impl` blocks and its
//! `#[wasm_bindgen]` surface live — see `pump.rs` for why that seam is a trait and not a
//! concrete type.

/// The self-rescheduling `requestAnimationFrame` loop.
pub mod pump;

pub use pump::FrameTarget;
pub use pump::RafPump;
