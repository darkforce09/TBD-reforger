//! Auth domain — session tokens ([`self::dev`]-login, Discord [`oauth`]) and the
//! self-service surface ([`me`]). T-934.15: the flat files moved here unchanged;
//! the same-named `auth.rs` is glob re-exported so `handlers::auth::*` paths hold.

// Deliberate inception: the domain keeps its same-named root handler file so every
// pre-T-934.15 `handlers::auth::…` path resolves through the glob re-export below.
#[allow(clippy::module_inception)]
mod auth;
pub use self::auth::*;

pub mod dev;
pub mod me;
pub mod oauth;
