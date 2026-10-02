//! The service record page and the panels it is built from.
//!
//! **Role:** declares the route component, the active-orders banner, the combat-history table,
//! the two leave panels and the column heading they share, and re-exports the page for the
//! router.
//! **Position:** the `/deployments` route, in the operations hub.
//! **Signals & state:** none at this level; the page owns the deployments fetch and each leave
//! panel owns its own.
//! **Invariants:** the page shows what the payload serves and an explicit empty affordance where
//! it serves nothing.
#![allow(dead_code)]

mod active_orders;
mod leave_of_absence;
mod leave_review_queue;
mod page;
mod service_record;
mod table_head;

pub use page::DeploymentsPage;

#[cfg(test)]
#[path = "tests/deployments.rs"]
mod tests;
