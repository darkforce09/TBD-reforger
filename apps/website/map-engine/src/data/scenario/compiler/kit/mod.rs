//! Role: Module boundary for mission/compiler/kit.
//! Position: `mission/compiler/kit` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;
mod aliases;
/// Expose aliases ::  kit aliases at this domain boundary.
pub use aliases::KitAliases;
/// Expose aliases :: load kit aliases at this domain boundary.
pub use aliases::load_kit_aliases;
#[cfg(test)]
mod tests;
