//! Role: the shared animation-frame pump and the trait of what it drives.
//! Position: `frame_pump` in the GPU frame crate.
//! Signals & state: the rAF callback slot, the disposal flag, the frame counter.
//! Invariants: the pump never names a renderer. It drives a [`pump::FrameTarget`], which the
//! renderer implements in its own crate — see `pump.rs` for why that seam is a trait and not a
//! concrete type.

/// The self-rescheduling `requestAnimationFrame` loop.
pub mod pump;

pub use pump::FrameTarget;
pub use pump::RafPump;
