//! TBD Reforger backend: the Axum REST API and SSE realtime hub behind the web platform.
//!
//! `core` holds the cross-cutting foundations (configuration, database, state, errors, auth
//! primitives, router, middleware, observability, and the shared HTTP/text/wire-format
//! primitives); `background_workers` holds the interval tasks the binary spawns at boot; the
//! eight domain modules each own the `/api/v1` route table for their surface, which
//! `core::http_router` merges; the remaining modules are the feature surfaces.

pub mod administration;
pub mod background_workers;
pub mod command_center;
pub mod community_content;
pub mod contract;
pub mod core;
pub mod handlers;
pub mod identity_and_access;
pub mod match_telemetry;
pub mod missions;
pub mod models;
pub mod operations;
pub mod server_infrastructure;
pub mod services;
