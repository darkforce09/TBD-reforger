//! Match telemetry: the game-server ingest of live server status and finished match results.
//!
//! **Role:** the API's match telemetry domain: its `/api/v1` route table ([`routes()`]), the
//! heartbeat, ingest and event-read handlers behind it, the registration, results-revision and
//! event-batch transactions, and the stored match, ingest wire and refusal models.
//! **Position:** above `api_state` (the application state every handler extracts),
//! `api_caller_identity` (the machine caller, the identity lock order), `api_http_layer` (the
//! `AuthUser` extractor), `api_member_activity` (attendance attribution, the statistics and the
//! leaderboard), `api_server_infrastructure` (the runtime-session fence, the status row and its
//! publisher), `api_audit_log`, `api_database`, `api_failpoints`, `api_foundation`,
//! `api_identifiers`, `api_mission_vocabulary` and `fleet_wire_contract`; names no other domain.
//! The API's router merges [`routes()`]; the operations domain reads the match models and the
//! integration suites call the ingest services.
//! **Signals & state:** none in memory; matches, their player lines, results revisions and
//! detailed events live in the database, reached through the caller's pool or transaction.
//! **Invariants:** every write is a `mod_runtime` machine credential acting for its own server;
//! a refusal a game runtime must act on is a 400 or 409 carrying `details.code`, never a 404; a
//! results revision applies only above the stored one, under the match row lock.

mod error;
pub mod handlers;
pub mod models;
pub mod prelude;
pub mod routes;
pub mod services;

pub use error::{Error, Result};
pub use routes::routes;
