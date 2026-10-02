//! Role: Module boundary for mission/extensions/authored.
//! Position: `mission/extensions/authored` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};
mod authored_block;
/// Expose authored block :: authored blocks at this domain boundary.
pub use authored_block::AUTHORED_BLOCKS;
/// Expose authored block ::  authored block at this domain boundary.
pub use authored_block::AuthoredBlock;
/// Expose authored block ::  authored blocks at this domain boundary.
pub use authored_block::AuthoredBlocks;
/// Expose authored block :: document owned blocks at this domain boundary.
pub use authored_block::DOCUMENT_OWNED_BLOCKS;
/// Expose authored block ::  extension blocks at this domain boundary.
pub use authored_block::ExtensionBlocks;
/// Expose authored block :: copy authored blocks at this domain boundary.
pub use authored_block::copy_authored_blocks;
/// Expose authored block :: is authored block at this domain boundary.
pub use authored_block::is_authored_block;
#[cfg(test)]
mod tests;
