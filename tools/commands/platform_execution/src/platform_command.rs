//! The `cargo xtask platform` command line.
//!
//! **Role:** [`PlatformCmd`], the clap subcommands of the platform group: the slice worktree
//! lifecycle, the preflight, the wave driver and the slice runner.
//! **Position:** parsed by the xtask binary inside its top-level command; [`crate::run`] receives
//! it.
//! **Signals & state:** none; plain data.
//! **Invariants:** `slice-worktree` and `wave` take their arguments raw (hyphens included), since
//! their drivers parse them.

use clap::Subcommand;
use std::path::PathBuf;

/// The `cargo xtask platform` subcommands.
#[derive(Subcommand, Debug)]
pub enum PlatformCmd {
    /// Slice worktree lifecycle: create, list, reap and merge per-ticket git worktrees.
    #[command(name = "slice-worktree", disable_help_flag = true)]
    SliceWorktree {
        /// `new <slice>` | `list` | `merge <slice>` | `drop <slice> [--force]` | `reap`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Assertions an unattended run must satisfy before it starts.
    Preflight {
        /// Report every finding but always exit 0.
        #[arg(long)]
        warn: bool,
    },
    /// Platform wave lifecycle: Rust slices, gated on cargo and trunk.
    ///
    /// The mod program has its own wave driver at `cargo xtask mod wave`, gated on the Enfusion
    /// compiler and a headless game boot. Same shape, different physics, so each lives under its
    /// own program group rather than one of them taking the bare verb.
    #[command(name = "wave", disable_help_flag = true)]
    Wave {
        /// `status` | `prep` | `gate [<base>|--slice <ticket>|--migrate-persist [audit|advance]]` |
        /// `test --slice <ticket> …` | `wave [--close]` | `verified <sha>` | `reclaim` |
        /// `land [--bookkeeping]` | `revert <sha>` | `push`  (default `status`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run ONE slice through the agent CLI and record its run receipt in the central ticket
    /// manager (`ttm record-run`). Exit-0-without-usage FAILS the run (no receipt, never
    /// tokens 0).
    #[command(name = "slice-run")]
    SliceRun {
        /// Ticket reference: slug or legacy number (executor must be claude-code)
        id: String,
        /// Replay mode: parse this recorded agent-CLI JSON instead of spawning
        #[arg(long)]
        fixture: Option<PathBuf>,
        /// Replay knob: fixed RFC 3339 UTC `started` stamp instead of now
        #[arg(long)]
        started: Option<String>,
        /// Print the plan, invoke nothing, record nothing
        #[arg(long)]
        dry_run: bool,
    },
}
