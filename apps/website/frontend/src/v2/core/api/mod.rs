//! Everything that talks to the backend: the HTTP client, the event stream, and the wire types.
//!
//! **Role:** groups the transport with the shapes it carries, so a page imports one path to fetch
//! and one path to name what comes back.
//! **Position:** the only door out of the browser. Nothing above this module builds a request.
//! **Signals & state:** the client owns the refresh single-flight cell and the peer-rotation
//! channel; the stream module owns the abort handle for the open connection.
//! **Invariants:** the wire types mirror the backend's models, and the backend wins on conflict.
//! The client and the stream are browser-only in their transport half, while the policy and the
//! wire types compile natively so both can be tested without a browser.

pub mod client;
pub mod dto;
pub mod sse;
