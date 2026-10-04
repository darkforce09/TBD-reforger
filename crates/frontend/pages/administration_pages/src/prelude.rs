//! The administration route components the app mounts, for `use administration_pages::prelude::*;`.
//!
//! **Role:** re-exports the seven route components of the administration hub.
//! **Position:** a re-export list over the crate's page modules; read by the app's route table.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every component keeps its home module. The components exist
//! only in the browser build, so they are re-exported only on `wasm32`.

#[cfg(target_arch = "wasm32")]
pub use crate::approvals::MissionApprovalsPage;
#[cfg(target_arch = "wasm32")]
pub use crate::audit_logs::AuditLogsPage;
#[cfg(target_arch = "wasm32")]
pub use crate::ballistics_catalogs::BallisticsCatalogsPage;
#[cfg(target_arch = "wasm32")]
pub use crate::content_manager::ContentManagerPage;
#[cfg(target_arch = "wasm32")]
pub use crate::event_manager::EventManagerPage;
#[cfg(target_arch = "wasm32")]
pub use crate::personnel::PersonnelRosterPage;
#[cfg(target_arch = "wasm32")]
pub use crate::server_control::ServerControlPage;
