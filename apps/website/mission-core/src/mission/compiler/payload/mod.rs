//! Role: Module boundary for mission/compiler/payload.
//! Position: `mission/compiler/payload` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::mission::orbat::derive_orbat_from_editor;
use serde_json::{Map, Value, json};
mod terrain_bounds;
/// Expose terrain bounds :: terrain bounds at this domain boundary.
pub use terrain_bounds::terrain_bounds;
mod serialization;
/// Expose payload :: known editor payload top level keys at this domain boundary.
pub use serialization::KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS;
/// Expose payload :: compile export at this domain boundary.
pub use serialization::compile_export;
/// Expose payload :: compile payload at this domain boundary.
pub use serialization::compile_payload;
/// Expose payload :: is known editor payload top level at this domain boundary.
pub use serialization::is_known_editor_payload_top_level;
mod export;
/// Expose export :: version body at this domain boundary.
pub use export::version_body;
/// Expose export :: version body to writer at this domain boundary.
pub use export::version_body_to_writer;
#[cfg(test)]
mod tests;
