use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum PlatformCmd {
    /// T-853: slice worktree lifecycle (port of scripts/mod/slice-worktree.sh)
    #[command(name = "slice-worktree", disable_help_flag = true)]
    SliceWorktree {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Unattended-run assertions (T-889 port of scripts/platform/preflight.sh)
    Preflight {
        /// Never exit non-zero (report only) — mirrors bash `--warn`
        #[arg(long)]
        warn: bool,
    },
    /// Platform wave lifecycle (T-853 port of scripts/platform/wave.sh).
    ///
    /// NOT `scripts/mod/wave.sh` — that is `cargo xtask mod wave` (T-890). Same shape, different
    /// physics; the two drivers get sibling names under their own program groups rather than one
    /// of them squatting the bare verb.
    #[command(name = "wave", disable_help_flag = true)]
    Wave {
        /// `status` | `prep` | `gate [<base>|--slice T-nnn|--migrate-persist [audit|advance]]` |
        /// `test --slice T-nnn …` | `wave [--close]` | `verified <sha>` | `reclaim` |
        /// `land [--bookkeeping]` | `revert <sha>` | `push` | `diff <arm>`  (default `status`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-913.2: run ONE slice through the agent CLI and write its run receipt under
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
