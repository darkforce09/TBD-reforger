//! Role: Module boundary for mission/extensions/radio.
//! Position: `mission/extensions/radio` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde_json::{Map, Value};
mod nets;
/// Expose nets ::  authored net at this domain boundary.
pub use nets::AuthoredNet;
/// Expose nets ::  authored radio plan at this domain boundary.
pub use nets::AuthoredRadioPlan;
/// Expose nets :: freq max mhz at this domain boundary.
pub use nets::FREQ_MAX_MHZ;
/// Expose nets :: freq min mhz at this domain boundary.
pub use nets::FREQ_MIN_MHZ;
/// Expose nets :: max label chars at this domain boundary.
pub use nets::MAX_LABEL_CHARS;
/// Expose nets :: max nets at this domain boundary.
pub use nets::MAX_NETS;
/// Expose nets :: ranges at this domain boundary.
pub use nets::RANGES;
/// Expose nets :: freq key at this domain boundary.
pub use nets::freq_key;
/// Expose nets :: parse at this domain boundary.
pub use nets::parse;
/// Expose nets :: refuse duplicate frequency at this domain boundary.
pub use nets::refuse_duplicate_frequency;
/// Expose nets :: validate at this domain boundary.
pub use nets::validate;
#[cfg(test)]
mod tests;
