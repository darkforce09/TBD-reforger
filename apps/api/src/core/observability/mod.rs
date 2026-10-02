//! Prometheus metrics and the health probe — everything that reports on the running API.
//!
//! * [`metrics_registry`] — a dependency-free Prometheus registry, one instance per application
//!   state ([`crate::core::application_state::AppState::metrics_registry`]), which the router and
//!   the background workers share (so tests are hermetic and there is no process-global mutable
//!   state).
//! * [`metrics_exposition`] — the 0.0.4 text format and `GET /metrics`, gated on the operator's
//!   `OBSERVABILITY_TOKEN` bearer ([`observability_auth`]).
//! * [`request_observer`] — the middleware that feeds the registry.
//! * [`health_probe`] — `GET /healthz`, a multi-check report that **can go red on either check
//!   independently**. A health probe that cannot fail is worse than none. The detail sits behind
//!   the same bearer; credential-less probers get `{"status": …}` and the 200/503 split.
//! * [`observability_auth`] — the `OBSERVABILITY_TOKEN` bearer check of both routes.

pub mod health_probe;
pub mod metrics_exposition;
pub mod metrics_registry;
pub mod observability_auth;
pub mod request_observer;
