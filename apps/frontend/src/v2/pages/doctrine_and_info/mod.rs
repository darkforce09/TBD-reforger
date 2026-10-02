//! The reference hub: the doctrine wiki, the vehicle index and the modpack manifests.
//!
//! **Role:** groups the three read-mostly pages the community consults — written doctrine,
//! vehicle identification, and the mod list a server expects a client to have.
//! **Position:** the `/wiki`, `/wiki/:slug`, `/vehicles` and `/modpacks` routes.
//! **Signals & state:** none at this level; each page owns its fetch and its signals.
//! **Invariants:** all three pages sit behind the authentication gate and share the split-view
//! primitives, so a change to that layout reaches every one of them.

pub mod modpacks;
pub mod vehicles;
pub mod wiki;
