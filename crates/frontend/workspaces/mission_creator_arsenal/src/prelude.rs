//! The items most callers name, for `use mission_creator_arsenal::prelude::*;`: the tab the
//! Attributes dialog mounts and the loadout conversions between a slot's `loadout` JSON and the
//! tab's picks.

#[cfg(target_arch = "wasm32")]
pub use crate::arsenal_tab::ArsenalTab;
pub use crate::loadout::{loadout_to_picks, picks_to_loadout};
