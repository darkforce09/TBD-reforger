//! Role: Module boundary for mission/extensions/objectives/win_conditions.
//! Position: `mission/extensions/objectives/win_conditions` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde::Serialize;
use serde_json::Value;
mod conditions;
/// Expose conditions :: authored modes at this domain boundary.
pub use conditions::AUTHORED_MODES;
/// Expose conditions ::  authored win conditions at this domain boundary.
pub use conditions::AuthoredWinConditions;
/// Expose conditions :: end on triggers at this domain boundary.
pub use conditions::END_ON_TRIGGERS;
/// Expose conditions :: fallback trigger at this domain boundary.
pub use conditions::FALLBACK_TRIGGER;
#[cfg(test)]
use conditions::PARAM_KEYS;
/// Expose conditions :: timeout minutes max at this domain boundary.
pub use conditions::TIMEOUT_MINUTES_MAX;
/// Expose conditions :: timeout minutes min at this domain boundary.
pub use conditions::TIMEOUT_MINUTES_MIN;
/// Expose conditions ::  win condition params at this domain boundary.
pub use conditions::WinConditionParams;
#[cfg(test)]
use conditions::mode_for_param_key;
/// Expose conditions :: optional param keys for mode at this domain boundary.
pub use conditions::optional_param_keys_for_mode;
/// Expose conditions :: param key for mode at this domain boundary.
pub use conditions::param_key_for_mode;
/// Expose conditions :: parse at this domain boundary.
pub use conditions::parse;
/// Expose conditions :: validate at this domain boundary.
pub use conditions::validate;
#[cfg(test)]
mod tests;
