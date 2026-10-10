//! The agent artifact tree.
//!
//! **Role:** the repository-relative paths of the pipeline output tree and the worktree base,
//! verified-commit marker and verdict folder inside it.
//! **Position:** the platform wave driver keeps its slice worktrees, gate verdicts and
//! verified-commit marker there, `xtask` its run records and `developer_tools` its export
//! operation logs.
//! **Signals & state:** none; constants.
//! **Invariants:** every path lies under [`ARTIFACTS_DIR`]; nothing there is an input to a gate.

/// Pipeline output: run reports, verify logs, handoff documents, export operation logs and the
/// worktree base. Nothing here is an input to a gate; everything is a record of a run.
pub const ARTIFACTS_DIR: &str = ".ai/artifacts";

/// Where a parallel ticket's git worktree is created, one folder per ticket id.
pub const WORKTREES_DIR: &str = ".ai/artifacts/worktrees";

/// The file holding the commit the last verifier examined, so the wave gate can report how many
/// commits of unverified debt stand behind the tip.
pub const LAST_VERIFIED_MARKER: &str = ".ai/artifacts/last-verified";

/// Recorded gate verdicts, one file per gate run.
pub const VERDICTS_DIR: &str = ".ai/artifacts/verdicts";
