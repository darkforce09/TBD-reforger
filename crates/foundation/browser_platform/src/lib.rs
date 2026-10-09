//! Browser console logging and same-origin HTTP GETs for wasm32 code.
//!
//! **Role:** the console macros (`console_log!`, `console_warn!`, `console_error!`) and the
//! fetch helpers (`fetch::fetch_bytes`, `fetch::fetch_text`, the streamed GET with its
//! `fetch::ByteProgress` callback, and the Range GET) that browser code shares. The names are code
//! spans rather than links because a native documentation build compiles the crate empty.
//! **Position:** foundation tier, wasm32 only, over `web-sys`, `js-sys`, `wasm-bindgen`,
//! `wasm-bindgen-futures` and `gloo-net`. The map engine's loaders and hosts call it; it knows no
//! map, mission or progress-bar vocabulary.
//! **Signals & state:** none; each call writes one console line or performs one request.
//! **Invariants:** the whole crate compiles only for `target_arch = "wasm32"`, so native
//! workspace builds never see a browser binding; a failed or non-2xx request is `None`, never an
//! empty body.
#![cfg(target_arch = "wasm32")]

pub mod console;
pub mod fetch;
pub mod prelude;
