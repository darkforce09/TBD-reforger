//! The platform factory behind `cargo xtask platform`: waves of Rust slices from worktree to main.
//!
//! **Role:** `wave_execution` is the wave driver (`platform wave`: status, worktree prep, the
//! tiered gates, per-slice landing, the wave close and the push); [`slice_execution`] runs one
//! slice through the agent CLI and writes its run receipt (`platform slice-run`, and `ticket run`
//! per ready slice); [`slice_worktree`] creates, lists, merges, drops and reaps the per-slice git
//! worktrees (`platform slice-worktree`, and the mod wave driver in-process); `preflight` asserts
//! what an unattended run needs before it starts (`platform preflight`). [`PlatformCmd`] is the
//! group's command line and [`run`] dispatches it.
//! **Position:** tier 7 of `tools/commands`, over `ci_task_catalog` (the schema gate list, the
//! member package list, the glibc stamp guard), the ticket crates, `process_runner`,
//! `repository_layout`, `repository_laws`, `verification_core` and `time_source`. The xtask
//! binary's `platform` group, its `ticket run` and its mod wave driver call it.
//! **Signals & state:** the wave driver and the preflight change the process's working directory
//! to the checkout root at entry; one thread-local buffer holds a gate step's captured output;
//! every child process runs through `process_runner`.
//! **Invariants:** a gate step that could not run never reads as a pass; `land` never merges a
//! slice without a passing gate verdict for its exact head; a run that reports no token usage
//! writes no receipt.

mod error;
mod platform_command;
mod platform_dispatch;
mod preflight;
pub mod prelude;
pub mod slice_execution;
pub mod slice_worktree;
mod wave_execution;

pub use error::{Error, Result};
pub use platform_command::PlatformCmd;
pub use platform_dispatch::run;
