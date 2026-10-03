//! The errors of the paper doll renderer.
//!
//! **Role:** the one error type of the crate: why the renderer could not create its GPU, draw a
//! frame, take a region state push, or finish its readback self-check.
//! **Position:** returned by the renderer's methods to the Arsenal host in the frontend, which
//! logs it or falls back to the flat paper doll.
//! **Signals & state:** none.
//! **Invariants:** each message starts with a stable kebab-case code (the GPU context's own codes,
//! `region-state-count`, `doll-probe-map-timeout`, `doll-probe-map-failed`) that logs and the
//! self-check readout match on.

/// Why the paper doll renderer could not create, draw, take states or self-check.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The GPU context of the canvas could not be created or hand out a frame.
    #[error(transparent)]
    Gpu(#[from] gpu_device::Error),

    /// A region state push did not carry exactly one byte per equipment region.
    #[error("region-state-count: expected {expected} region state bytes, got {got}")]
    RegionStateCount {
        /// The number of equipment regions.
        expected: usize,
        /// The number of bytes the push carried.
        got: usize,
    },

    /// The self-check's readback buffer did not map within its polling budget.
    #[error("doll-probe-map-timeout")]
    ProbeMapTimeout,

    /// The self-check's readback buffer failed to map.
    #[error("doll-probe-map-failed")]
    ProbeMapFailed,
}

/// The result of a paper doll renderer call.
pub type Result<T, E = Error> = std::result::Result<T, E>;
