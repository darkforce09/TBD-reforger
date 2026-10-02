use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum PlatformCmd {
    /// Slice worktree lifecycle: create, list, reap and merge per-ticket git worktrees.
    #[command(name = "slice-worktree", disable_help_flag = true)]
    SliceWorktree {
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
        /// `status` | `prep` | `gate [<base>|--slice T-nnn|--migrate-persist [audit|advance]]` |
        /// `test --slice T-nnn …` | `wave [--close]` | `verified <sha>` | `reclaim` |
        /// `land [--bookkeeping]` | `revert <sha>` | `push`  (default `status`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run ONE slice through the agent CLI and write its run receipt under
    /// `.ai/tickets/metrics/<id>/`. Exit-0-without-usage FAILS the run (no file, never
    /// tokens 0). `ticket run` delegates here per ready slice.
    #[command(name = "slice-run")]
    SliceRun {
        /// Ticket id (executor must be claude-code)
        id: String,
        /// Replay mode: parse this recorded agent-CLI JSON instead of spawning
        #[arg(long)]
        fixture: Option<PathBuf>,
        /// Replay knob: fixed RFC 3339 UTC `started` stamp instead of now
        #[arg(long)]
        started: Option<String>,
        /// Print the plan, invoke nothing, write nothing
        #[arg(long)]
        dry_run: bool,
    },
}
