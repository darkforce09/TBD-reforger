//! The administration hub: the screens only administrators can reach.
//!
//! **Role:** groups the six restricted screens — the operations calendar, server control, the
//! personnel roster, the mission approval queue, the content manager and the audit trail.
//! **Position:** the `/admin/*` routes, rendered inside the navigation frame.
//! **Signals & state:** each page owns its own fetches and signals; nothing is shared here.
//! **Invariants:** every page in this hub renders behind the administrator gate, and every request
//! it makes is a browser-only path — a native compile resolves each fetch to nothing.

pub mod approvals;
pub mod audit_logs;
pub mod content_manager;
pub mod event_manager;
pub mod personnel;
pub mod server_control;

#[allow(unused_imports)]
pub use approvals::MissionApprovalsPage;
#[allow(unused_imports)]
pub use audit_logs::AuditLogsPage;
#[allow(unused_imports)]
pub use content_manager::ContentManagerPage;
#[allow(unused_imports)]
pub use event_manager::EventManagerPage;
#[allow(unused_imports)]
pub use personnel::PersonnelRosterPage;
#[allow(unused_imports)]
pub use server_control::ServerControlPage;
