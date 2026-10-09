//! The two linear congruential generators whose exact streams are pinned by callers.
//!
//! **Role:** [`LinearCongruential64`], the 64-bit generator with Knuth's MMIX constants, and
//! [`LinearCongruential32`], the 32-bit generator with the C standard library's `rand`
//! constants. Each state step is `state × multiplier + increment`, wrapping, and the draw is the
//! new state.
//! **Position:** re-exported at the crate root. New code draws from [`crate::SplitMix64`]; these
//! two exist because seeded outputs that are stored or pinned come from their streams: the
//! mission document's seeded slot positions and the render diagnostics' stress scene, whose
//! first instances are pinned bit for bit, and its compute cull self-check field.
//! **Signals & state:** one state word per generator, advanced by every draw.
//! **Invariants:** integer arithmetic only, so a seed determines the stream on every machine and
//! target; [`LinearCongruential32::next_unit`] divides 24 bits by `2^24`, which is exact in
//! `f32`. The low bits of an LCG cycle with short periods, so callers draw from the high bits.
//! Not cryptographic.

/// A 64-bit linear congruential generator with Knuth's MMIX multiplier and increment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinearCongruential64 {
    state: u64,
}

impl LinearCongruential64 {
    /// Knuth's MMIX multiplier.
    pub const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

    /// Knuth's MMIX increment.
    pub const INCREMENT: u64 = 1_442_695_040_888_963_407;

    /// A generator whose first draw is `state × MULTIPLIER + INCREMENT`.
    pub const fn new(state: u64) -> Self {
        Self { state }
    }

    /// The next state, which is the draw: `state × MULTIPLIER + INCREMENT`, wrapping.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        self.state
    }
}

/// A 32-bit linear congruential generator with the C standard library's `rand` constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinearCongruential32 {
    state: u32,
}

impl LinearCongruential32 {
    /// The C standard library's `rand` multiplier.
    pub const MULTIPLIER: u32 = 1_103_515_245;

    /// The C standard library's `rand` increment.
    pub const INCREMENT: u32 = 12_345;

    /// A generator whose first draw is `state × MULTIPLIER + INCREMENT`.
    pub const fn new(state: u32) -> Self {
        Self { state }
    }

    /// The next state, which is the draw: `state × MULTIPLIER + INCREMENT`, wrapping.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        self.state
    }

    /// The next draw in `[0, 1)`: the top 24 bits divided by `2^24`, exact in `f32`.
    pub fn next_unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }
}

#[cfg(test)]
#[path = "tests/linear_congruential.rs"]
mod tests;
