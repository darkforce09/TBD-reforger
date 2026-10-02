//! The editor's full-screen dialogs — the surfaces that overlay the whole workspace rather than
//! frame the map or edit one selection.
//!
//! **Role:** groups the four dialogs the Mission Creator raises over its canvas: the controls
//! hint and shortcut reference, the mission settings sheet, the faction template authoring
//! dialog and the ORBAT graph authoring dialog.
//! **Position:** a leaf of `v2::apps::editor::ui`, beside the docked chrome, the outliner and the
//! inspectors. None of these dialogs is routed; every one of them is mounted from
//! `mission_editor`, so they are editor surfaces and never pages.
//! **Signals & state:** each dialog takes an `open` signal from its mount site and owns only the
//! draft state of the thing it edits. Document changes leave through the map engine's hosted
//! commands, exactly as every other editor surface does.
//! **Invariants:** a dialog that binds a key installs a window-level `keydown` of its own and is
//! therefore censused by [`help_modal::keymap_census`], which adjudicates every editor binding
//! against every other. A dialog that stacks over another takes its overlay z from
//! `core::ui::modal_stack` and gates Escape on being topmost, so the stack unwinds in order.

/// The faction template dialog: side, name and role templates with an optional kind-only
/// loadout, plus the vehicle pool, wired to the owner-scoped factions CRUD.
pub mod faction_manager;
/// The help surface: the shortcut reference behind the top strip's Help menu, the toggleable
/// controls hint, and the keymap census that proves the reference documents every live binding.
pub mod help_modal;
/// The ORBAT authoring dialog: the live mission-doc graph of sides, squads and slots, with the
/// faction template apply/save path and the vehicle pool beside it.
pub mod orbat_manager;
/// The mission settings sheet: the flow, win-condition, task, radio, weather, audio and spawn
/// cards that edit the open mission's own configuration.
pub mod settings_modal;
