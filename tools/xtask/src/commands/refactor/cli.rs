//! The `cargo xtask refactor` command group's clap declarations.
//!
//! **Role:** declares [`RefactorCmd`] and the `relocate` flags in [`RelocateArgs`].
//!
//! **Position:** named by `TopCmd::Refactor` in [`crate::cli`]; matched by [`super::dispatch`].
//!
//! **Signals & state:** none; declarations only.
//!
//! **Invariants:** exactly one of `--dry-run`, `--apply` and `--verify` is accepted per run
//! (clap's required `mode` group).

use std::path::PathBuf;

use clap::{ArgGroup, Args, Subcommand};

#[derive(Subcommand, Debug)]
pub(crate) enum RefactorCmd {
    /// Move tracked files and rewrite every reference to them, from a relocation manifest.
    ///
    /// `--dry-run` prints and verifies the plan, `--apply` writes it (only when its dry run would
    /// pass) and verifies it, `--verify` checks that no live file still spells what a manifest
    /// retired (every committed manifest when none is named). Exit 0 clean, 1 findings, 2 did not
    /// run.
    Relocate(RelocateArgs),
}

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .args(["dry_run", "apply", "verify"])
))]
pub(crate) struct RelocateArgs {
    /// The manifest: a TSV file of `path`, `rust_path` and `text` rows.
    #[arg(long, value_name = "path.tsv")]
    pub(crate) manifest: Option<PathBuf>,
    /// Print the moves and rewrites the manifest asks for and verify the tree they would leave;
    /// write nothing.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Make the moves and rewrites, then verify the manifest.
    #[arg(long)]
    pub(crate) apply: bool,
    /// Check that no live file spells a retired path or Rust prefix.
    #[arg(long)]
    pub(crate) verify: bool,
}
