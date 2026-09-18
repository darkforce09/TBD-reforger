//! Prometheus metrics and the health probe — everything that reports on the running API.
//!
//! * [`metrics_registry`] — a dependency-free Prometheus registry, one instance per
//!   [`crate::core::http_router::router`] call (so tests are hermetic and there is no
//!   process-global mutable state).
//! * [`metrics_exposition`] — the 0.0.4 text format and `GET /metrics`, gated on the same
//!   `X-Service-Token` the game-server ingest uses.
//! * [`request_observer`] — the middleware that feeds the registry.
//! * [`health_probe`] — `GET /healthz`, a multi-check report that **can go red on either check
//!   independently**. A health probe that cannot fail is worse than none. The detail sits behind
//!   `X-Service-Token`; credential-less probers get `{"status": …}` and the 200/503 split.

pub mod health_probe;
pub mod metrics_exposition;
pub mod metrics_registry;
pub mod request_observer;
