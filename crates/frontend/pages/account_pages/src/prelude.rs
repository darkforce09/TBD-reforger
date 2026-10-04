//! The account pages' routed components, for `use account_pages::prelude::*;`.
//!
//! **Role:** re-exports the three route components the app's route table mounts.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module; the route components exist
//! only on `wasm32`, so they are re-exported only there.

#[cfg(target_arch = "wasm32")]
pub use crate::auth_callback::AuthCallbackPage;
#[cfg(target_arch = "wasm32")]
pub use crate::login::LoginPage;
#[cfg(target_arch = "wasm32")]
pub use crate::settings::SettingsPage;
