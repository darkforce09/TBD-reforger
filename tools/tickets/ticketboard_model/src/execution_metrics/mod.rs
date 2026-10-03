//! Measured run receipts and historical token estimates, kept structurally apart.
//!
//! **Role:** scans `.ai/tickets/metrics/` run receipts into per-ticket and per-agent aggregates
//! ([`measured`]) and `.ai/tickets/estimates/` files into per-class and per-domain aggregates
//! ([`estimated`]).
//! **Position:** over [`crate::ticket_registry`] and `ticket_metrics`; the Metrics tab and the
//! detail panel of `apps/ticketboard` paint [`models::MetricsView`] and emit
//! [`events::MetricsEvent`]s.
//! **Signals & state:** none; plain data rebuilt on load.
//! **Invariants:** measured and estimated figures are never summed together; missing data never
//! renders as a zero; a malformed file is a named error row, never skipped.

pub mod estimated;
pub mod events;
pub mod measured;
pub mod models;
