//! The API's Discord clients: OAuth2 and guild-member reads, and the announcement webhook.
//!
//! **Role:** everything the API sends to Discord over HTTP, with the typed failures it reports,
//! so no domain owns a Discord client another domain needs.
//! **Position:** above `api_foundation` (the embed field caps), `api_http_layer` (the outbound
//! retry on `429`, the reconciliation outcome the metrics count) and `api_identifiers`; the
//! application state holds the clients, the OAuth handlers and the membership reconciliation of
//! `api_identity_and_access` and the announcement push of `api_community_content` call them, and the
//! `staging-fixtures` host tool reads guild members through them.
//! **Signals & state:** none here; see each client.
//! **Invariants:** no file here names a domain; every failure is a typed [`Error`], never a
//! message assembled by the caller.

pub mod discord_client;
pub mod discord_user_profile;
pub mod discord_webhook;
pub mod error;
pub mod membership_lookup_failure;
pub mod prelude;

pub use error::{Error, Result};
