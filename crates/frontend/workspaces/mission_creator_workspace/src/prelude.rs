//! The items most callers name, for `use mission_creator_workspace::prelude::*;`: the two route
//! components the app's route table mounts.

#[cfg(target_arch = "wasm32")]
pub use crate::{MissionEditorPage, ReviewWorkspacePage};
