//! The offline service worker binary.
//!
//! **Role:** the service worker of the single-page app: it precaches the app document at
//! install, deletes the caches of earlier builds at activation, and answers every intercepted
//! request by the policy of the `offline_cache_policy` crate, slicing `Range` requests for map
//! assets out of the cached body by [`range_slicing`].
//! **Position:** the shell layer of the frontend crates, a peer of `frontend_application` with no
//! Cargo edge between the two in either direction. Trunk builds this binary from the app's
//! `index.html` as a `no-modules` worker next to the app bundle (`offline_service_worker.js` and
//! `offline_service_worker_bg.wasm` at the site root);
//! `crates/frontend/shell/frontend_application/service_worker.js` imports it, registers the three
//! event listeners and calls the exported `on_install`, `on_activate` and `on_fetch`.
//! **Signals & state:** no Rust state; everything durable lives in Cache Storage under the names
//! of `offline_cache_policy::cache_names`.
//! **Invariants:** the WebAssembly half exists only on `wasm32`; the native build is an empty
//! `main`, so workspace builds and host lints compile the binary without a browser; the native
//! test build compiles the pure [`range_slicing`] arithmetic alone, and `cargo test` checks it.

#[cfg(target_arch = "wasm32")]
mod cached_range_response;
#[cfg(target_arch = "wasm32")]
mod fetch_handling;
#[cfg(target_arch = "wasm32")]
mod lifecycle_events;
#[cfg(any(target_arch = "wasm32", test, doc))]
mod range_slicing;
#[cfg(target_arch = "wasm32")]
mod worker_scope;

/// Empty on every target: the worker's work starts from the exported event handlers.
fn main() {}
