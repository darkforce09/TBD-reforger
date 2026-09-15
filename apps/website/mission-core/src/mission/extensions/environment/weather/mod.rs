//! Role: Module boundary for mission/extensions/environment/weather.
//! Position: `mission/extensions/environment/weather` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde_json::{Map, Value};
mod timeline;
/// Expose timeline ::  authored keyframe at this domain boundary.
pub use timeline::AuthoredKeyframe;
/// Expose timeline ::  authored weather timeline at this domain boundary.
pub use timeline::AuthoredWeatherTimeline;
/// Expose timeline :: weather presets at this domain boundary.
pub use timeline::WEATHER_PRESETS;
/// Expose timeline :: minutes strictly increase at this domain boundary.
pub use timeline::minutes_strictly_increase;
/// Expose timeline :: parse at this domain boundary.
pub use timeline::parse;
/// Expose timeline :: validate at this domain boundary.
pub use timeline::validate;
#[cfg(test)]
mod tests;
