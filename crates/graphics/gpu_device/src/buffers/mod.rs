//! Role: the bookkeeping of long-lived GPU buffers: per-lane pooled vertex buffers and the
//! one-mapping-at-a-time readback guard.
//! Position: `buffers` in the GPU device crate; the map engine's render engine owns a pool, and
//! its readback probes and the frame timer guard their mappings with a lane.
//! Signals & state: the pool's buffers and host copies, the readback cells.
//! Invariants: allocation and mapping arithmetic only; nothing here knows what the bytes depict.

/// Per-lane pooled vertex buffers that grow and never shrink.
pub mod pool;

/// The one-mapping-at-a-time guard of a readback buffer.
pub mod readback;
