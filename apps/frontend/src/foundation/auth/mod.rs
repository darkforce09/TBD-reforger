//! Session state: who is signed in, how the token pair is kept alive, and what a route requires.
//!
//! **Role:** groups the session data, the store that holds it reactively, the session refresh that
//! keeps a single-use refresh token from being spent twice, the sign-out hooks, the route guard and
//! the content gates.
//! **Position:** above the transport, whose wire types (`User`, `Role`, the rotated token pair) the
//! session is built from and whose `TokenProvider` the store implements; below the pages, the
//! workspaces and the shell, which reach the layers above the session only through the sign-out
//! hooks they register. Provided once at the application root; every page reads the session
//! through it.
//! **Signals & state:** the store owns the session signals. The per-tab single-flight cell, the
//! peer-rotation channel, the sign-out hook registry and the persisted blob live outside the
//! reactive system.
//! **Invariants:** refresh tokens are single-use — the backend rotates and revokes on every call,
//! so several callers wanting a refresh at once must share one attempt or all but the first are
//! refused, wrongly ending the session. The access token is never written to durable storage.

#[cfg(any(target_arch = "wasm32", test))]
pub mod gates;
#[cfg(any(target_arch = "wasm32", test))]
pub mod logout_hooks;
pub mod refresh_transaction;
pub mod route_guard;
pub mod session;
pub mod session_identity;
#[cfg(target_arch = "wasm32")]
pub mod session_refresh;
pub mod session_restore;
pub mod store;

#[cfg(target_arch = "wasm32")]
pub use gates::AdminGate;
#[cfg(target_arch = "wasm32")]
pub use gates::AuthGate;
#[cfg(test)]
pub use route_guard::route_auth_redirect;
#[cfg(target_arch = "wasm32")]
pub use session::AUTH_PERSIST_KEY;
#[cfg(target_arch = "wasm32")]
pub use session::{load_persisted, persist};
#[cfg(any(target_arch = "wasm32", test))]
pub use store::AuthStore;

#[cfg(test)]
#[path = "tests/auth.rs"]
mod tests;

/// The session refresh is browser-only, so its source pins are a native test module of their own.
#[cfg(test)]
#[path = "tests/session_refresh.rs"]
mod session_refresh_source_pins;
