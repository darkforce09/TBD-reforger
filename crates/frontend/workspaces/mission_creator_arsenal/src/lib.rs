//! The Mission Creator's Arsenal: the Attributes dialog's tab where a mission maker edits one
//! slot's persisted loadout.
//!
//! **Role:** owns the Arsenal tab (`ArsenalTab`) and everything it draws and decides beyond the
//! rules: the loaded catalog view and its action handlers, the panels around a slot's loadout
//! (the doll host with its SVG fallback, the compatibility panel, the cargo editor), the 3D paper
//! doll host, the pure loadout core (the loadout JSON, the export and import gates, the
//! copy-and-apply buffer and the receipts) and the loadout writes to the mission document.
//! **Position:** the fourth Mission Creator crate: above `mission_creator_session`,
//! `mission_creator_engine_bridge`, `mission_creator_state` and the foundation crates; below the
//! workspace, whose Attributes dialog mounts `ArsenalTab`. The loadout rows, the compatibility
//! feed and the weight arithmetic it reads sit in `mission_creator_state::arsenal_rules`.
//! **Signals & state:** the tab's signals live per mounted tab ([`arsenal_tab`]); the copy buffer
//! lives in the hosted commands, outside this crate; the doll's renderer handle lives with the
//! doll panel.
//! **Invariants:** a pick reaches the document only through `loadout_commands` and the hosted
//! commands, and the history tail runs only on an acknowledged write. A module that touches
//! `web_sys` or a live document handle is `#[cfg(target_arch = "wasm32")]` and its `pub mod` line
//! carries the same gate; [`loadout`] stays free of `web_sys`, so the native tests cover it.

/// The Arsenal tab component, the reactive state its loaded view shares and the persistence lines
/// it states.
pub mod arsenal_tab;
/// The 3D paper-doll mount over the engine's doll renderer, wasm-only like every other live engine
/// host.
#[cfg(target_arch = "wasm32")]
pub mod doll;
/// The crate's error: why a loadout is refused before it reaches a file or the mission document.
pub mod error;
/// The pure loadout core: serialization, the export and import gates, the buffer verbs and the
/// receipts.
pub mod loadout;
/// The Arsenal's writes to the mission document: one slot's loadout, the buffer applied across a
/// selection, and the strip. Reaches the live document, so wasm32-only.
#[cfg(target_arch = "wasm32")]
pub mod loadout_commands;
/// The panels the Arsenal draws around a slot's loadout: the doll host with its SVG fallback, the
/// compatibility panel with its attachment toggles, and the cargo editor.
pub mod panels;
/// The items most callers name, for `use mission_creator_arsenal::prelude::*;`.
pub mod prelude;
/// The loaded catalog view and its action handlers.
mod tab_content;

#[cfg(target_arch = "wasm32")]
pub use arsenal_tab::ArsenalTab;
pub use loadout::{
    ImportedLoadout, apply_receipt, commit_one_write, copy_receipt, loadout_to_picks,
    picks_to_loadout, plan_apply, plan_remove, refusal_line, remove_receipt, try_export,
    try_import,
};
