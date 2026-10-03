//! TBD Reforger backend: the Axum REST API and SSE realtime hub behind the web platform.
//!
//! **Role:** the thin application the API crates under `crates/api/` are assembled into:
//! [`router`] merges the eight domain route tables under `/api/v1` and wraps them in the global
//! middleware chain, and [`composition`] builds the application state with its concrete services.
//! **Position:** the top of the API: above the kernel crates (`api_foundation`,
//! `api_configuration`, `api_database`, `api_failpoints`, `api_http_layer`, `api_audit_log`,
//! `api_mission_vocabulary`, `api_discord`, `api_equipment_datasets`, `api_caller_identity`,
//! `api_member_activity`, `api_state`), the domain crates (`api_community_content`,
//! `api_identity_and_access`, `api_administration`, `api_server_infrastructure`, `api_missions`,
//! `api_match_telemetry`, `api_operations`, `api_command_center`) and `api_background_workers`.
//! The `api` binary serves the router and arms the workers; the `import-registry` binary and the
//! integration suites under `tests/` build their state and router here.
//! **Signals & state:** none of its own; the state [`composition::application_state`] returns
//! owns everything it builds.
//! **Invariants:** no domain logic lives here: a route belongs to its domain's table, and this
//! crate only merges the tables; the layout rules in `src/tests/` hold the split.

pub mod composition;
pub mod router;

#[cfg(test)]
#[path = "tests/architecture_rules.rs"]
mod architecture_rules;

#[cfg(test)]
#[path = "tests/prose_rules.rs"]
mod prose_rules;
