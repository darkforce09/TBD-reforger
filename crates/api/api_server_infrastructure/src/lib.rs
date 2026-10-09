//! Server infrastructure: the dedicated-server registry, its live status feed, the machine
//! credentials, the fleet command ledger, the game-runtime sessions and the fleet scenarios.
//!
//! **Role:** the API's server infrastructure domain: its `/api/v1` route table ([`routes()`]),
//! the handlers behind it, the credential, session, command ledger and status publishing
//! services, and the server, status, command, credential and scenario models.
//! **Position:** above `api_state` (the application state every handler extracts),
//! `api_caller_identity` (the machine caller, the session authorization of the status stream,
//! the administrator recheck), `api_http_layer` (the caller extractors, the event stream
//! authorization, the token primitives, the realtime hub), `api_community_content` (the modpack
//! a server requires), `api_audit_log`, `api_foundation`, `api_identifiers`,
//! `api_mission_vocabulary` and `fleet_wire_contract`; names no other domain. The API's router
//! merges [`routes()`]; the background workers, the match telemetry, missions, operations and
//! command center domains, the `staging-fixtures` host tool and the integration suites call its
//! services.
//! **Signals & state:** none in memory beyond the realtime hub it publishes to; servers,
//! statuses, credentials, commands, runtime sessions and fleet scenarios live in the database,
//! reached through the caller's pool or transaction.
//! **Invariants:** operators reach a game host only through a ledger command its executor claims;
//! a machine credential acts only for its own server and executor kind; every `server:{id}`
//! publish serialises one [`models::server::ServerStatus`] shape.

mod error;
pub mod handlers;
pub mod models;
pub mod prelude;
pub mod routes;
pub mod services;

pub use error::{Error, Result};
pub use routes::routes;
