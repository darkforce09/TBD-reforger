//! Repository discovery and the corpus load.
//!
//! **Role:** declares `corpus_loading` and `discovery`.
//! **Position:** used by `crate::application_state` and by the desktop application's start-up.
//! **Signals & state:** none here; see each module.
//! **Invariants:** the corpus load is all or nothing.

pub mod corpus_loading;
pub mod discovery;
