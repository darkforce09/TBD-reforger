//! Mission approvals: the submissions waiting on a reviewer, the artifact under review, and the
//! decision made on it.
//!
//! **Role:** declares the route component with the desk its queue and drawer share, the pending
//! queue, the review drawer with the mission's briefing and the artifact's provenance and history,
//! the decision form, and the words a refused decision is told in.
//! **Position:** the `/admin/approvals` route, in the administration hub.
//! **Signals & state:** none at this level; the page owns the fetch and the desk.
//! **Invariants:** the queue and the drawer read the same fetched page, so the drawer can never
//! show a submission the queue is no longer listing, and every decision names the artifact of the
//! review it was made on.
#![allow(dead_code)]

mod decision_refusal;
mod page;
mod review_briefing;
mod review_decision;
mod review_drawer;
mod submission_queue;

pub use page::MissionApprovalsPage;

#[cfg(test)]
#[path = "tests/approvals.rs"]
mod tests;
