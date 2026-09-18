//! Cross-cutting HTTP foundations: router assembly, the global middleware chain, and the
//! observability surfaces every route is measured by.

pub mod http_router;
pub mod middleware;
pub mod observability;
