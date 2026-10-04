//! The account pages: sign-in, the page the sign-in redirect lands on, and the account settings.
//!
//! **Role:** groups the pages that act on the viewer's own session rather than on mission or
//! operations data.
//! **Position:** a page crate above the foundation crates; the app's route table mounts its
//! `/login`, `/auth/callback` and `/settings` routes. Sign-in and the callback render bare,
//! outside the navigation frame; the settings page renders inside it.
//! **Signals & state:** the pages here read and write the shared authentication store.
//! **Invariants:** these routes must stay reachable while signed out — the callback page in
//! particular runs before a session exists. The route components are compiled for `wasm32` only,
//! because they drive the browser's location and call endpoints that exist only in the browser
//! build.

pub mod auth_callback;
pub mod login;
pub mod prelude;
pub mod settings;
