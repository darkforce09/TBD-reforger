//! The lane projection and the borrowed view of the Waves tab.
//!
//! **Role:** declares `view` and `wave_projection`.
//! **Position:** built by `crate::application_state::workspace_state` from a loaded lock; painted by
//! the desktop application's Waves tab.
//! **Signals & state:** none here; see each module.
//! **Invariants:** lanes keep the lock's order and membership.

pub mod view;
pub mod wave_projection;
