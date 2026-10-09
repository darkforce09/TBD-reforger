//! The single-page app's session: who is signed in, how the token pair is kept alive, and what a
//! route requires.
//!
//! **Role:** the session data and its persisted slice, the reactive store that holds it, the
//! session refresh that keeps a single-use refresh token from being spent twice, the sign-out
//! hooks, the route guard and the two content gates.
//! **Position:** a foundation crate above `frontend_transport`, whose `TokenProvider` the store
//! implements, `frontend_api_dtos` (the profile, the role ladder, the rotated token pair),
//! `frontend_route_table` (the access tiers the guard enforces) and `frontend_ui` (the toast the
//! guard raises); below the features, the pages, the workspaces and the app shell, which reach
//! the layers above the session only through the sign-out hooks they register. Provided once at
//! the application root; every page reads the session through it.
//! **Signals & state:** the store owns the session signals. The per-tab single-flight cell, the
//! peer-rotation channel, the sign-out hook registry and the persisted blob live outside the
//! reactive system.
//! **Invariants:** refresh tokens are single-use — the backend rotates and revokes on every call,
//! so several callers wanting a refresh at once must share one attempt or all but the first are
//! refused, wrongly ending the session. The access token is never written to durable storage.
//! Only the code that calls the browser (storage, the cross-tab lock, the router hooks, the
//! refresh request) is gated to the wasm32 build.

pub mod gates;
pub mod logout_hooks;
pub mod prelude;
pub mod refresh_transaction;
pub mod route_guard;
pub mod session;
pub mod session_identity;
#[cfg(target_arch = "wasm32")]
pub mod session_refresh;
pub mod session_restore;
pub mod store;

pub use gates::{AdminGate, AuthGate};
pub use route_guard::route_auth_redirect;
pub use session::AUTH_PERSIST_KEY;
#[cfg(target_arch = "wasm32")]
pub use session::{load_persisted, persist};
pub use store::AuthStore;

#[cfg(test)]
#[path = "tests/auth.rs"]
mod tests;
