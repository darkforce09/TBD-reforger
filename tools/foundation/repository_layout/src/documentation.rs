//! The documentation tree root more than one tool names.
//!
//! **Role:** the root of the committed documentation tree.
//! **Position:** `xtask` and the documentation checks judge the tree's links and placement;
//! `developer_tools` resolves citations across it; the relocation tool finds its frozen areas
//! under it. A document only one tool names stays in that tool's own layout module.
//! **Signals & state:** none; constants.
//! **Invariants:** the root is relative and has no trailing slash.

/// Root of the committed documentation tree.
pub const DOCUMENTATION_ROOT: &str = "documentation";
