//! Cross-cutting foundations every feature module rests on: runtime configuration, the database
//! connection lifecycle, shared application state, the single handler error type, the realtime
//! SSE hub, authentication primitives, router assembly, the global middleware chain, and the
//! observability surfaces every route is measured by.

pub mod application_state;
pub mod authentication_primitives;
pub mod configuration;
pub mod database;
pub mod error_handling;
pub mod http_router;
pub mod middleware;
pub mod observability;
pub mod realtime_hub;
