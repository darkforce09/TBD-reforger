//! Repository discovery, corpus loading and the ticket models every feature reads.
//!
//! **Role:** declares `models` (the corpus, its refusal and the shared ticket projections) and
//! `services` (root discovery and the all-or-nothing corpus load).
//! **Position:** the shared base of the crate's features; `crate::application_state` loads through
//! it and the desktop application resolves its root with it.
//! **Signals & state:** none here; see each module.
//! **Invariants:** imports no consuming feature, only `core` besides itself.

pub mod models;
pub mod services;
