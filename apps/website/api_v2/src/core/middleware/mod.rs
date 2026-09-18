//! Cross-cutting request middleware owned by the core layer.
//!
//! The in-memory limiters, CORS, request-id and logging layers still live in
//! `crate::middleware`; this module holds the pieces whose state is durable rather than
//! per-process.

pub mod durable_ratelimit;
