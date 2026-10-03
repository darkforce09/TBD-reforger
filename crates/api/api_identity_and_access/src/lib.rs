//! Identity and access: Discord OAuth2 sign-in, session token rotation and logout, the caller's
//! own profile (`/me`), and the game-account link handshake.
//!
//! **Role:** the API's identity and access domain: its `/api/v1` route table ([`routes()`]), the
//! handlers behind it, the account, session and Discord membership services, and the account row
//! models.
//! **Position:** above `api_state` (the application state every handler extracts),
//! `api_caller_identity` (the session and account authority, the identity locks, the machine
//! caller), `api_discord` (the Discord clients), `api_member_activity`, `api_audit_log`,
//! `api_http_layer`, `api_configuration`, `api_foundation` and `api_identifiers`; names no other
//! domain. The API's router merges [`routes()`]; the background workers, the other domains, the
//! `staging-fixtures` host tool and the integration suites call its services.
//! **Signals & state:** none in memory; accounts, sessions, refresh tokens, link codes and
//! membership snapshots live in the database, reached through the caller's pool or transaction.
//! **Invariants:** an account's authority comes from the verified Discord membership snapshot,
//! never from `users.role`; a refresh token is spent once, and a replayed one revokes the
//! account's sessions; the development login is registered only when `routes` is told the
//! configuration is a development one.

mod error;
pub mod handlers;
pub mod models;
pub mod prelude;
pub mod routes;
pub mod services;

pub use error::{Error, Result};
pub use routes::routes;
