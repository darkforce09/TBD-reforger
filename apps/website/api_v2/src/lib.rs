//! TBD Reforger backend: the Axum REST API and SSE realtime hub behind the web platform.
//!
//! `core` holds the cross-cutting foundations (configuration, database, state, errors, auth
//! primitives, router, middleware, observability, and the shared HTTP/text/wire-format
//! primitives); `background_workers` holds the interval tasks the binary spawns at boot; the
//! eight domain modules — `administration`, `command_center`, `community_content`,
//! `identity_and_access`, `match_telemetry`, `missions`, `operations`, and
//! `server_infrastructure` — each own the `/api/v1` route table for their surface, which
//! `core::http_router` merges.

pub mod administration;
pub mod background_workers;
pub mod command_center;
pub mod community_content;
pub mod core;
pub mod identity_and_access;
pub mod match_telemetry;
pub mod missions;
pub mod operations;
pub mod server_infrastructure;

#[cfg(test)]
#[path = "tests/architecture_rules.rs"]
mod architecture_rules;

#[cfg(test)]
#[path = "tests/prose_rules.rs"]
mod prose_rules;
