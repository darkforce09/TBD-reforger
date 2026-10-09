//! The deterministic pseudo-random generators of the workspace.
//!
//! **Role:** [`SplitMix64`], a seeded stream of 64-bit draws with unit, range and index helpers,
//! for every place that needs "random" values that come out the same on every run, with its
//! increment and finaliser for callers that step or hash words of their own; and the two linear
//! congruential generators [`LinearCongruential64`] and [`LinearCongruential32`], kept because
//! seeded outputs that are stored or pinned come from their exact streams.
//! **Position:** foundation tier, depending on nothing. The mission editing scatter, the
//! ballistics agreement lattice, the loadout Apply draw, the staging load pacing and the world
//! line-of-sight bench draw from [`SplitMix64`]; the mission document's seeded slots and the render
//! diagnostics' stress scene and compute cull field from the linear congruential generators.
//! **Signals & state:** each generator owns one state word, advanced by every draw.
//! **Invariants:** a seed determines the whole stream on every machine and target: integer
//! arithmetic only, and the one float conversion is exact. Not cryptographic.

mod linear_congruential;
pub mod prelude;
mod split_mix_64;

pub use linear_congruential::{LinearCongruential32, LinearCongruential64};
pub use split_mix_64::SplitMix64;
