//! The SplitMix64 generator.
//!
//! **Role:** a 64-bit counter stepped by the golden-ratio increment `0x9E37_79B9_7F4A_7C15`
//! ([`SplitMix64::INCREMENT`]) and passed through the published SplitMix64 finaliser
//! ([`SplitMix64::finalise`]: shifts 30, 27, 31; multipliers `0xBF58_476D_1CE4_E5B9` and
//! `0x94D0_49BB_1331_11EB`), plus draws derived from it. The increment and the finaliser are
//! public for callers that hash a word or step a Weyl sequence of their own.
//! **Position:** re-exported at the crate root as [`crate::SplitMix64`].
//! **Signals & state:** one `u64` counter per generator, advanced by every draw.
//! **Invariants:** seed 0 yields the published reference stream `0xE220_A839_7B1D_CDAF`,
//! `0x6E78_9E6A_A1B9_65F4`, `0x06C4_5D18_8009_454F`, …; [`SplitMix64::next_unit`] scales the
//! top 53 bits by `2^-53`, and since a power-of-two scale of an integer below `2^53` is exact,
//! multiplying by `2^-53` and dividing by `2^53` give the same bits on every target.

/// The SplitMix64 generator: a 64-bit counter stepped by the golden-ratio increment and mixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// The golden-ratio increment the counter advances by before every draw: an odd 64-bit
    /// constant, so the counter is a Weyl sequence that visits every 64-bit value once per period.
    pub const INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;

    /// A generator whose first draw mixes `seed + 0x9E37_79B9_7F4A_7C15`.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The SplitMix64 finaliser of `word`: an avalanche mix and a bijection on 64-bit words, so
    /// words that differ in one bit come out differing everywhere. It is the draw of a generator
    /// whose counter sits one [`SplitMix64::INCREMENT`] below `word`, and a mix rather than a
    /// source of entropy.
    #[must_use]
    pub const fn finalise(word: u64) -> u64 {
        let mut mixed = word;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// The next 64-bit draw.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(Self::INCREMENT);
        Self::finalise(self.state)
    }

    /// The next draw in `[0, 1)`: the top 53 bits scaled by `2^-53`, exact on every target.
    pub fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// The next draw in `[low, low + span)`.
    pub fn next_in(&mut self, low: f64, span: f64) -> f64 {
        low + span * self.next_unit()
    }

    /// The next draw in `0..bound`; a `bound` of zero draws `0`.
    pub fn next_index(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound.max(1)
    }
}

#[cfg(test)]
#[path = "tests/split_mix_64.rs"]
mod tests;
