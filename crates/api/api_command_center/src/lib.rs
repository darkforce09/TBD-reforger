//! Command center: the platform dashboard, community leaderboards and per-player statistics.
//!
//! **Role:** the API's command center domain: its `/api/v1` route table ([`routes()`]), the
//! dashboard, leaderboard and statistics card handlers behind it, and the fleet overview service
//! the dashboard reads.
//! **Position:** above `api_state` (the application state every handler extracts),
//! `api_http_layer` (the `AuthUser` extractor), the five domains it names
//! (`api_community_content`, `api_identity_and_access`, `api_missions`, `api_operations`,
//! `api_server_infrastructure`), `api_foundation`, `api_identifiers` and `fleet_wire_contract`.
//! The API's router merges [`routes()`]; the integration suites call the leaderboard handler.
//! **Signals & state:** none in memory; every figure is read from the database through the pool
//! the application state holds.
//! **Invariants:** every route takes `AuthUser`; the domain owns no table and no model of its own,
//! and only reads the rows other domains write; a leaderboard category maps to its ordering only
//! through a fixed list.

mod error;
pub mod handlers;
pub mod prelude;
pub mod routes;
pub mod services;

pub use error::{Error, Result};
pub use routes::routes;
