//! The documents more than one tool names.
//!
//! **Role:** the documentation tree root and the two ticket-domain documents `ticket sync`
//! rewrites between markers.
//! **Position:** `ticket_registry` writes the two documents and walks the tree; `xtask` judges the
//! tree's links and placement; `developer_tools` resolves citations across it;
//! `ticketboard_desktop` watches the two documents. A document only one tool names stays in that
//! tool's own layout module.
//! **Signals & state:** none; constants.
//! **Invariants:** every path lies under [`DOCUMENTATION_ROOT`].

/// Root of the committed documentation tree.
pub const DOCUMENTATION_ROOT: &str = "documentation";

/// The Mission Creator roadmap carrying the generated "recommended next work" block that
/// `ticket sync` injects between its markers.
pub const ROADMAP: &str =
    "documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md";

/// The Eden gap-analysis table whose ticket column `ticket sync` keeps in step with the registry.
pub const GAP_ANALYSIS: &str = "documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/eden_gap_analysis.md";
