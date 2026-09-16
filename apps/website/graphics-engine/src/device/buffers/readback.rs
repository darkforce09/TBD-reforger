//! Role: readback.
//! Position: `device/buffers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::cell::Cell;

/// One buffer's `map_async` lifecycle: at most one outstanding mapping, plus whether the value last read out of it is still trustworthy.
#[derive(Debug, Default)]
pub struct ReadbackLane {
    in_flight: Cell<bool>,
    has_sample: Cell<bool>,
}

impl ReadbackLane {
    /// A lane with no mapping outstanding and nothing read yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim the lane for one `map_async`.
    pub fn begin(&self) -> bool {
        if self.in_flight.get() {
            return false;
        }
        self.in_flight.set(true);
        true
    }

    /// Settle a completed `map_async` callback. `mapped_ok` is `res.is_ok()`.
    pub fn settle(&self, mapped_ok: bool) -> bool {
        self.in_flight.set(false);
        if !mapped_ok {
            self.has_sample.set(false);
        }
        mapped_ok
    }

    /// Record that a value was read out of the mapped range — call from inside the success arm, after the read. Separate from [`Self::settle`] because settling is about the *mapping* and this is about the *value*: a callback that maps successfully and then declines to read (a short buffer, a shape it does not recognise) has no sample to offer.
    pub fn record_sample(&self) {
        self.has_sample.set(true);
    }

    /// In flight.
    #[must_use]
    pub fn in_flight(&self) -> bool {
        self.in_flight.get()
    }

    /// Is the value last read out of this lane this-frame-fresh? `false` after a failed readback, so a HUD renders "no reading" rather than a number from an earlier frame.
    #[must_use]
    pub fn has_sample(&self) -> bool {
        self.has_sample.get()
    }
}

#[cfg(test)]
#[path = "tests/readback_tests.rs"]
mod tests;
