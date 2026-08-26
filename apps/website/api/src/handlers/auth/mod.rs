//! Auth domain — session tokens ([`self::dev`]-login, Discord [`oauth`]) and the
//! self-service surface ([`me`]). T-934.15: the flat files moved here unchanged;
//! the same-named `auth.rs` is glob re-exported so `handlers::auth::*` paths hold.

mod auth;
pub use self::auth::*;

pub mod dev;
pub mod me;
pub mod oauth;
