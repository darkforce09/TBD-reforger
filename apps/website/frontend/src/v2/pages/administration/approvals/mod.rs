//! Mission approvals: the submissions waiting on a reviewer, and the decision made on each.
//!
//! **Role:** declares the route component, the pending queue, and the review drawer beside it.
//! **Position:** the `/admin/approvals` route, in the administration hub.
//! **Signals & state:** none at this level; the page owns the fetch and the selection.
//! **Invariants:** the queue and the drawer read the same fetched page, so the drawer can never
//! show a submission the queue is no longer listing.
#![allow(dead_code)]

mod page;
mod review_drawer;
mod submission_queue;

pub use page::MissionApprovalsPage;
