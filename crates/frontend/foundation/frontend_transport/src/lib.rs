//! The single-page app's door to the API: the HTTP client, the typed endpoint calls, and the
//! server-sent event streams.
//!
//! **Role:** the request verbs with their refresh policy and the crate's one error, one typed call
//! per route that has one, the live server status stream and the audit log stream, and the token
//! provider seam the session implements.
//! **Position:** the only door out of the browser. Nothing above this crate builds a request. It
//! sits below the session: every authenticated call takes the session as a
//! [`token_provider::TokenProvider`] (declared here, implemented by the session store), so the
//! transport never imports the session. The wire types come from `frontend_api_dtos`.
//! **Signals & state:** the stream modules own the abort handles of their open connections; the
//! refresh single-flight cell and the peer-rotation channel are the token provider's.
//! **Invariants:** exactly one refresh-and-retry per request. Only code that calls the browser
//! directly (the request verbs, the stream readers) is gated to the wasm32 build; the error, the
//! refresh policy, the endpoint paths and the frame readers compile natively, so all of
//! them are tested without a browser.

pub mod audit_stream;
pub mod client;
pub mod endpoints;
pub mod error;
pub mod prelude;
pub mod sse;
pub mod sse_frames;
pub mod token_provider;

pub use error::{Error, Result};

/// The production text of this area's source files, for the guard tests that pin it.
#[cfg(test)]
#[path = "tests/source_pins.rs"]
pub(crate) mod source_pins;
