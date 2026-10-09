//! Missions: the mission library and the editor payloads the Mission Creator saves, the version
//! history, the compiled artifacts, the reviews and the approval queue, the mission deployments,
//! the armory, the faction and asset registries, and the game-runtime mission reads.
//!
//! **Role:** the API's missions domain: its `/api/v1` route table ([`routes()`]), the handlers
//! behind it, the schema contract and write-boundary validation of every mission document, the
//! compile, artifact, review, deployment and registry import services, and the mission, review,
//! deployment, faction and registry models.
//! **Position:** above `api_state` (the application state every handler extracts),
//! `api_caller_identity` (the mission maker and administrator rechecks, the machine caller),
//! `api_http_layer` (the caller extractors, the realtime hub), `api_server_infrastructure` (the
//! fleet command ledger a deployment runs on), `api_community_content` (the modpack a registry
//! belongs to), `api_audit_log`, `api_configuration`, `api_database`,
//! `api_foundation`, `api_identifiers`, `api_mission_vocabulary`, the mission crates
//! (`mission_compiler`, `mission_model`, `mission_validation`, `mission_wire_safety`) and
//! `fleet_wire_contract`; names no other domain. The API's router
//! merges [`routes()`]; the background workers, the operations and command center domains, the
//! `import-item-registry` tool and the integration suites call its services.
//! **Signals & state:** none in memory beyond the schema validators, compiled once per process;
//! missions, versions, artifacts, reviews, deployments, factions and registries live in the
//! database, reached through the caller's pool or transaction.
//! **Invariants:** a review decides exactly the artifact it was opened on; a deployment loads an
//! approved artifact; every stored compiled document passes `mission.schema.json`.

pub mod contract;
mod error;
pub mod handlers;
pub mod models;
pub mod prelude;
pub mod routes;
pub mod services;
pub mod validation;

pub use error::{Error, Result};
pub use routes::routes;
