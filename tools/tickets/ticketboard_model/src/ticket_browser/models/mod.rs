//! The board, tree, detail-section and view models of the browser.
//!
//! **Role:** declares `detail_sections`, `program_tree`, `status_board` and `view`.
//! **Position:** built by `crate::application_state::workspace_state` at load; painted by the
//! desktop application's browser views.
//! **Signals & state:** none here; see each module.
//! **Invariants:** the projections are built once per load, so painting only reads.

pub mod detail_sections;
pub mod program_tree;
pub mod status_board;
pub mod view;
