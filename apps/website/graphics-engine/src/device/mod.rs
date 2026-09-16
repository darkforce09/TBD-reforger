//! Role: device.
//! Position: `apps/website/graphics-engine/src` — GPU resource ownership.
//! Signals & state: buffer pools and readback fences.
//! Invariants: allocation arithmetic only. Nothing here knows what the bytes depict.

/// Buffer pools and readback fences.
pub mod buffers;
