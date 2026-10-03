//! Everything that talks to the backend: the HTTP client, the event stream, and the wire types.
//!
//! **Role:** groups the transport with the shapes it carries, so a page imports one path to fetch
//! and one path to name what comes back, and — for the routes that have them — one typed call per
//! route.
//! **Position:** the only door out of the browser. Nothing above this module builds a request. It
//! sits below the session: every authenticated call takes the session as a `TokenProvider`
//! (declared here, implemented by the session store), so the transport never imports the session.
//! **Signals & state:** the stream module owns the abort handle for the open connection; the
//! refresh single-flight cell and the peer-rotation channel are the token provider's.
//! **Invariants:** the wire types mirror the backend's models, and the backend wins on conflict.
//! The client, the endpoint calls and the stream are browser-only in their transport half, while
//! the policy, the endpoint paths and the wire types also compile into the native test build, so
//! all of them can be tested without a browser.

#[cfg(any(target_arch = "wasm32", test))]
pub mod audit_stream;
pub mod client;
pub mod dto;
pub mod endpoints;
pub mod sse;
#[cfg(any(target_arch = "wasm32", test))]
pub mod sse_frames;
#[cfg(target_arch = "wasm32")]
pub mod token_provider;
