//! The reference hub: the doctrine wiki, the vehicle index and the modpack manifests.
//!
//! **Role:** groups the three read-mostly pages the community consults — written doctrine,
//! vehicle identification, and the mod list a server expects a client to have.
//! **Position:** a page crate above the foundation crates (`frontend_session`,
//! `frontend_transport`, `frontend_api_dtos`, `frontend_ui`); the app's route table mounts its
//! pages at `/wiki`, `/wiki/:slug`, `/vehicles` and `/modpacks`.
//! **Signals & state:** none at this level; each page owns its fetch and its signals.
//! **Invariants:** all three pages sit behind the authentication gate and share the split-view
//! primitives, so a change to that layout reaches every one of them. The route components fetch
//! in the browser only, so they exist on `wasm32` alone; the pure readers under them compile on
//! every target for the native tests.

pub mod modpacks;
pub mod prelude;
pub mod vehicles;
#[cfg(any(target_arch = "wasm32", test))]
pub mod wiki;
