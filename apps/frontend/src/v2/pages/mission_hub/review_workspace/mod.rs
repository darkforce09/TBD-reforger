//! The read-only review workspace: the Scenario Creator opened on exactly the version an artifact
//! compiled from, under a banner that says so.
//!
//! **Role:** declares the route component and the banner it lays over the editor.
//! **Position:** the `/missions/:id/artifacts/:artifact_id/workspace` route, in the mission hub.
//! **Signals & state:** none at this level; the route owns the read and opens the editor's review
//! mode.
//! **Invariants:** the workspace is the editor itself in its read-only review mode, not a copy of
//! it, so a reviewer inspects the version with every tool the author had and saves nothing.
#![allow(dead_code)]

mod banner;
mod page;

pub use page::ReviewWorkspacePage;

#[cfg(test)]
#[path = "tests/review_workspace.rs"]
mod tests;
