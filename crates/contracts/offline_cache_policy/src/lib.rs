//! The offline policy shared by the service worker and the single-page app.
//!
//! **Role:** decides what the offline service worker decides — which cache holds a response, how
//! each request is served, when a saved copy answers instead of the network, and which terrain
//! files make up the offline pack — as pure functions with no browser type.
//! **Position:** contracts tier, depending on no workspace crate. Consumed by the
//! `offline_service_worker` app, which applies the policy through `web-sys` inside the worker, and
//! by `frontend`, which downloads the offline pack into the same caches from the page.
//! **Signals & state:** none; pure functions and plain data.
//! **Invariants:** the worker and the page derive cache names and pack entries from the same
//! functions, so both always read and write the same caches under the same keys.

pub mod cache_names;
pub mod error;
pub mod network_fallback;
pub mod offline_pack;
pub mod prelude;
pub mod request_classification;
pub mod terrain_id;

pub use error::{Error, Result};
pub use terrain_id::TerrainId;
