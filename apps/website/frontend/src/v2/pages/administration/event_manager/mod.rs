//! The operations calendar: scheduling, editing and cancelling the unit's operations.
//!
//! **Role:** declares the route component, the state every panel reads, the month grid and day
//! panel, the schedule and edit forms, the mission pickers they share, the two destructive
//! confirmations, and the access panel that administers one operation's policies, groups, places
//! and participant evidence.
//! **Position:** the `/admin/events` route, in the administration hub.
//! **Signals & state:** none at this level; the page builds the state and hands it down.
//! **Invariants:** every panel takes the same copyable state handle rather than a parameter list,
//! so there is exactly one source for the calendar's position, the two forms' fields and the three
//! fetches behind them.
#![allow(dead_code)]

mod access;
mod confirm_dialogs;
mod dates;
mod edit_dialog;
mod event_table;
mod lifecycle;
mod mission_picker;
mod page;
mod schedule_dialog;
mod state;

pub use page::EventManagerPage;

#[cfg(test)]
use lifecycle::DELETE_EVENT_CONFIRM_DESC;

#[cfg(test)]
#[path = "tests/event_manager.rs"]
mod tests;
