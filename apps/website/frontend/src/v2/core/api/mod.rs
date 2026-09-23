//! Everything that talks to the backend: the HTTP client, the event stream, and the wire types.
//!
//! **Role:** groups the transport with the shapes it carries, so a page imports one path to fetch
//! and one path to name what comes back, and — for the routes that have them — one typed call per
//! route.
//! **Position:** the only door out of the browser. Nothing above this module builds a request.
//! **Signals & state:** the client owns the refresh single-flight cell and the peer-rotation
//! channel; the stream module owns the abort handle for the open connection.
//! **Invariants:** the wire types mirror the backend's models, and the backend wins on conflict.
//! The client, the endpoint calls and the stream are browser-only in their transport half, while
//! the policy, the endpoint paths and the wire types compile natively so all of them can be tested
//! without a browser.

pub mod client;
pub mod dto;
pub mod endpoints;
pub mod sse;
