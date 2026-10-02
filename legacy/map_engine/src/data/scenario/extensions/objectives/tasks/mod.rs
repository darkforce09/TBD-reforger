//! Role: Module boundary for mission/extensions/objectives/tasks.
//! Position: `mission/extensions/objectives/tasks` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
mod hierarchy;
/// Expose hierarchy ::  authored task at this domain boundary.
pub use hierarchy::AuthoredTask;
/// Expose hierarchy :: legal transitions at this domain boundary.
pub use hierarchy::LEGAL_TRANSITIONS;
/// Expose hierarchy :: states at this domain boundary.
pub use hierarchy::STATES;
/// Expose hierarchy ::  schedule at this domain boundary.
pub use hierarchy::Schedule;
/// Expose hierarchy :: tiers at this domain boundary.
pub use hierarchy::TIERS;
/// Expose hierarchy ::  task state at this domain boundary.
pub use hierarchy::TaskState;
/// Expose hierarchy ::  task tier at this domain boundary.
pub use hierarchy::TaskTier;
/// Expose hierarchy :: is legal transition at this domain boundary.
pub use hierarchy::is_legal_transition;
/// Expose hierarchy :: parse at this domain boundary.
pub use hierarchy::parse;
/// Expose hierarchy :: transition at this domain boundary.
pub use hierarchy::transition;
/// Expose hierarchy :: validate at this domain boundary.
pub use hierarchy::validate;
/// Expose hierarchy :: validate schedule at this domain boundary.
pub use hierarchy::validate_schedule;
/// Expose hierarchy :: window is legal at this domain boundary.
pub use hierarchy::window_is_legal;
#[cfg(test)]
mod tests;
