//! T-934.4 — mission library & management surfaces.

// create_mission_dialog.rs renamed at the T-934.4 move (audit table 3.5).
pub mod create_dialog;
// missions.rs renamed at the T-934.4 move (audit table 3.5): the module is the
// library browser, not the missions domain.
pub mod mission_library;
pub mod mission_overview;
