//! The administration hub: the screens only administrators can reach.
//!
//! **Role:** groups the seven restricted screens — the operations calendar, server control, the
//! personnel roster, the mission approval queue, the content manager, the audit trail and the
//! ballistics catalogs.
//! **Position:** a page crate above the foundation crates (`frontend_api_dtos`,
//! `frontend_transport`, `frontend_session`, `frontend_ui`, `frontend_route_table`) and the
//! `mission_review_record` feature; the app's route table mounts its `/admin/*` route components,
//! rendered inside the navigation frame.
//! **Signals & state:** each page owns its own fetches and signals; nothing is shared here.
//! **Invariants:** every page in this hub renders behind the administrator gate, and every request
//! it makes is a browser-only path — the route components and the views that fetch exist only in
//! the `wasm32` build, while the wording, the view models and the request builders under them
//! compile on every target for the native tests.

pub mod approvals;
pub mod audit_logs;
pub mod ballistics_catalogs;
pub mod content_manager;
pub mod event_manager;
pub mod personnel;
pub mod prelude;
pub mod server_control;

/// The production text of this area's source files, for the guard tests that pin it.
#[cfg(test)]
#[path = "tests/source_pins.rs"]
mod source_pins;
