//! TBD Reforger backend: the Axum REST API and SSE realtime hub behind the web platform.
//!
//! `core` holds the cross-cutting foundations (configuration, database, state, errors, auth
//! primitives, router, middleware, observability); `background_workers` holds the interval
//! tasks the binary spawns at boot; the remaining modules are the feature surfaces.

pub mod background_workers;
pub mod contract;
pub mod core;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
