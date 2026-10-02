//! Cross-cutting foundations every feature module rests on: runtime configuration, the database
//! connection lifecycle, shared application state, the single handler error type, the realtime
//! SSE hub, authentication primitives, router assembly, the global middleware chain, the
//! observability surfaces every route is measured by, the process-wide shutdown signal, and the
//! shared primitives the feature modules build on — request pagination, outbound HTTP retry, text
//! handling, and the JSON wire formats — plus the failpoints the test builds arm, which a deploy
//! build compiles out.

pub mod application_state;
pub mod authentication_primitives;
pub mod configuration;
pub mod database;
pub mod error_handling;
pub mod failpoints;
pub mod http;
pub mod http_client;
pub mod http_router;
pub mod middleware;
pub mod observability;
pub mod process_lifecycle;
pub mod realtime_hub;
pub mod text;
pub mod wire_format;
