//! Product capabilities that more than one page or workspace shows, above the shared foundations.
//!
//! **Role:** holds the self-contained features the routed pages and the workspaces share — a
//! feature renders one product concept (its views, controls and wording) and fetches through
//! `foundation`, so every surface that shows the concept shows it the same way.
//! **Position:** the features layer (foundation < features < pages, workspaces < shell). It imports
//! from `foundation` and the map engine; pages, workspaces and the shell import from it.
//! **Signals & state:** none at this level; each feature's components own their own signals.
//! **Invariants:** a feature never imports a page, a workspace or the shell
//! (`cargo xtask verify frontend-layering`).

/// A mission's review record: history, thread, comment composer, artifact provenance, the
/// submission control and the wording they share.
pub mod mission_review_record;
