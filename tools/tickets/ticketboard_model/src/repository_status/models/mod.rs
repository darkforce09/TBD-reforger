//! The strict-check and `git status` models and the banner view.
//!
//! **Role:** declares `check_status`, `git_status` and `view`.
//! **Position:** held and fed by the desktop application, which spawns the check and `git status`;
//! painted by its status banner.
//! **Signals & state:** none here; see each module.
//! **Invariants:** only an observed exit 0 of the strict check is green.

pub mod check_status;
pub mod git_status;
pub mod view;
