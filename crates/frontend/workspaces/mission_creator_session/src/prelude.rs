//! The items most callers name, for `use mission_creator_session::prelude::*;`: the writer-role
//! gate every authoring surface asks, the payload-size estimate the toolbelt reads, and the
//! hydrated mission row the settings dialog mirrors.

#[cfg(target_arch = "wasm32")]
pub use crate::document_commands::{HydratedRow, hydrated_row};
pub use crate::mission_size::estimate_compiled_bytes;
pub use crate::tab_lock::may_write;
