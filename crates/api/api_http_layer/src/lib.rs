//! The API's HTTP layer: everything a request passes through on its way to a handler.
//!
//! **Role:** HS256 access tokens and the opaque-token primitives ([`authentication_primitives`]),
//! the global middleware chain with the authentication extractors, the client resolution behind
//! the trusted proxies and the two rate-limit tiers ([`middleware`]), the Prometheus registry,
//! `/metrics` and `/healthz` ([`observability`]), the in-process SSE hub ([`realtime_hub`]) and
//! the outbound retry policy ([`http_client`]).
//! **Position:** above `api_foundation`, `api_configuration` and `api_identifiers`; `api_state`'s
//! `AppState` holds its services, the API router (`api::router`) mounts its layers, and the kernel
//! and domain crates take its extractors. Every middleware and extractor reads a sub-state through
//! `FromRef`, so nothing here names the application state or a domain.
//! **Signals & state:** the rate limiters' buckets, the metrics registry and the hub's broadcast
//! channel are the shared mutable state; each lives in a value the application state owns, never
//! in a process-global.
//! **Invariants:** authentication is an extractor, never a layer, so the tier a route requires
//! travels with its handler; an access token is accepted only HS256-signed, unexpired, with the
//! platform's issuer and audience and a nonempty subject and session; an opaque token is stored
//! only as its SHA-256 hex and compared in constant time; the durable rate limiter fails closed.

pub mod authentication_primitives;
mod error;
pub mod http_client;
pub mod middleware;
pub mod observability;
pub mod prelude;
pub mod realtime_hub;

pub use error::{Error, Result};
