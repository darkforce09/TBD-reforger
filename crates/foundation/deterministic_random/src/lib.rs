//! The deterministic pseudo-random generator of the workspace.
//!
//! **Role:** [`SplitMix64`], a seeded stream of 64-bit draws with unit, range and index helpers,
//! for every place that needs "random" values that come out the same on every run.
//! **Position:** foundation tier, depending on nothing. The mission editing scatter and the
//! ballistics agreement lattice draw from it.
//! **Signals & state:** the generator owns one `u64` of state, advanced by every draw.
//! **Invariants:** a seed determines the whole stream on every machine and target: integer
//! arithmetic only, and the one float conversion is exact. Not cryptographic.

pub mod prelude;
mod split_mix_64;

pub use split_mix_64::SplitMix64;
