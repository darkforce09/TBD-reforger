//! Role: Module boundary for mission/extensions/environment/audio.
//! Position: `mission/extensions/environment/audio` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde_json::{Map, Value};
mod emitters;
/// Expose emitters ::  authored audio at this domain boundary.
pub use emitters::AuthoredAudio;
/// Expose emitters ::  authored emitter at this domain boundary.
pub use emitters::AuthoredEmitter;
/// Expose emitters ::  authored music cue at this domain boundary.
pub use emitters::AuthoredMusicCue;
/// Expose emitters :: music events at this domain boundary.
pub use emitters::MUSIC_EVENTS;
/// Expose emitters :: parse at this domain boundary.
pub use emitters::parse;
/// Expose emitters :: radius above zero at this domain boundary.
pub use emitters::radius_above_zero;
/// Expose emitters :: validate at this domain boundary.
pub use emitters::validate;
#[cfg(test)]
mod tests;
