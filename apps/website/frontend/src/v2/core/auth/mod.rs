//! Session state: who is signed in, how the token pair is kept alive, and what a route requires.
//!
//! **Role:** groups the session data, the store that holds it reactively, the single-flight cell
//! that keeps a single-use refresh token from being spent twice, the route guard, and the guard on
//! links the app renders.
//! **Position:** provided once at the application root; every page reads the session through it.
//! **Signals & state:** the store owns the session signals. The single-flight cell and the
//! persisted blob live outside the reactive system.
//! **Invariants:** refresh tokens are single-use — the backend rotates and revokes on every call,
//! so several callers wanting a refresh at once must share one attempt or all but the first are
//! refused, wrongly ending the session. The access token is never written to durable storage.

pub mod refresh_transaction;
pub mod role;
pub mod route_guard;
pub mod session;
pub mod session_identity;
pub mod session_restore;
pub mod single_flight;
pub mod store;
pub mod url_guard;

#[allow(unused_imports)]
pub use role::{has_min_role, has_min_role_authed, Role};
#[allow(unused_imports)]
pub use route_guard::route_auth_redirect;
#[allow(unused_imports)]
pub use session::{
    from_persist_json, to_persist_json, PersistState, PersistedAuth, RefreshResponse, Session,
    User, AUTH_PERSIST_KEY,
};
#[cfg(target_arch = "wasm32")]
pub use session::{load_persisted, persist};
pub use single_flight::SingleFlight;
pub use store::AuthStore;

#[cfg(test)]
#[path = "tests/auth.rs"]
mod tests;
