//! The command centre: the three read-only screens that report the unit's current situation.
//!
//! **Role:** groups the landing dashboard, the live game-server panel and the announcement
//! board. None of them writes platform data; each renders one fetched payload.
//! **Position:** a page crate above the foundation crates (`frontend_session`,
//! `frontend_transport`, `frontend_api_dtos`, `frontend_ui`); the app's route table mounts its
//! pages at `/`, `/server-intel`, `/announcements` and `/announcements/:id`, the first section of
//! the navigation registry, rendered inside the frame.
//! **Signals & state:** each page owns its own resource and signals; nothing is shared here.
//! **Invariants:** every page in this hub sits behind the sign-in gate and fetches only from
//! the browser build, so the route components exist on `wasm32` alone; the pure helpers under
//! them compile for the native tests.

pub mod announcements;
pub mod dashboard;
pub mod prelude;
pub mod server_intel;

/// The production text of this area's source files, for the guard tests that pin it.
#[cfg(test)]
#[path = "tests/source_pins.rs"]
mod source_pins;
