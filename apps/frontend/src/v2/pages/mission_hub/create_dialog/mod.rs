//! The new-mission dialog, and the source guards on the request it builds.
//!
//! **Role:** declares the dialog and re-exports it for the mission library, which is the only
//! surface that opens it.
//! **Position:** the mission hub; no route of its own.
//! **Signals & state:** none at this level; the dialog owns every field signal.
//! **Invariants:** the create request carries only fields the endpoint accepts.
#![allow(dead_code)]

mod dialog;

pub use dialog::CreateMissionDialog;

#[cfg(test)]
#[path = "tests/dialog_t671_create_carries_the_briefing.rs"]
mod t671_create_carries_the_briefing;
