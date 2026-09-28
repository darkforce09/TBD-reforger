//! The offline policy shared by the service worker and the single-page app.
//!
//! **Role:** decides everything the offline service worker decides — which cache holds a
//! response, how each request is served, how a `Range` header maps onto a cached body, and which
//! terrain files make up the offline pack — as pure functions with no browser type.
//! **Position:** consumed by this crate's `offline_service_worker` binary, which applies the
//! policy through `web-sys` inside the worker, and by `website-frontend`, which downloads the
//! offline pack into the same caches from the page. Depends on no workspace crate.
//! **Signals & state:** none; pure functions and plain data.
//! **Invariants:** the worker and the page derive cache names and pack entries from the same
//! functions, so both always read and write the same caches under the same keys.

pub mod cache_names;
pub mod network_fallback;
pub mod offline_pack;
pub mod range_slicing;
pub mod request_classification;
