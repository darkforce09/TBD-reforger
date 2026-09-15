//! Role: Module boundary for mission/extensions/tactical_graphics.
//! Position: `mission/extensions/tactical_graphics` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde_json::{Map, Value};
mod shapes;
/// Expose shapes ::  authored tactical graphic at this domain boundary.
pub use shapes::AuthoredTacticalGraphic;
/// Expose shapes :: brushes at this domain boundary.
pub use shapes::BRUSHES;
/// Expose shapes :: kinds at this domain boundary.
pub use shapes::KINDS;
/// Expose shapes :: max points at this domain boundary.
pub use shapes::MAX_POINTS;
/// Expose shapes ::  tactical graphic style at this domain boundary.
pub use shapes::TacticalGraphicStyle;
/// Expose shapes :: min points at this domain boundary.
pub use shapes::min_points;
/// Expose shapes :: parse at this domain boundary.
pub use shapes::parse;
/// Expose shapes :: validate at this domain boundary.
pub use shapes::validate;
mod quote;
use quote::quote;
use quote::type_name;
#[cfg(test)]
mod tests;
