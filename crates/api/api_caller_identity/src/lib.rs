//! Caller identity: who an API request acts as, and with what authority.
//!
//! **Role:** the role ladder, the account authority read, the session authorization, the
//! identity and account lock order, and the machine caller with its extractor: everything a
//! domain needs to know about its caller without depending on the identity or server domains.
//! **Position:** above `api_http_layer` (the access token claims, the `AuthUser` extractor and the
//! `SessionAuthority` trait), `api_configuration`, `api_foundation` and `api_identifiers`; the
//! [`session_authorization::DatabaseSessionAuthority`] it implements is built by the API's
//! composition root and held by the application state; its checks are called by every domain
//! that authorizes a caller inside a transaction, by the `staging-fixtures` host tool and by the
//! integration suites; names no domain.
//! **Signals & state:** none in memory; sessions, accounts, membership snapshots and machine
//! credentials are read from the database on the caller's pool or connection.
//! **Invariants:** permissions come from the configured guild's verified membership snapshot,
//! never from `users.role`; identity and account locks are always taken sorted; a machine
//! credential acts only as its own executor for its own server.

pub mod account_authority;
pub mod arma_identity_link;
pub mod cached_membership_permissions;
mod error;
pub mod identity_ownership;
pub mod machine_authentication;
pub mod machine_caller;
pub mod prelude;
pub mod session_authorization;
pub mod user_role;

pub use error::{Error, Result};
pub use user_role::UserRole;
