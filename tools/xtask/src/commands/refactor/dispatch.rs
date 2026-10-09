//! Routes a `cargo xtask refactor` command to its mode.
//!
//! **Role:** resolves the checkout root and runs `relocate` in the mode its flags chose.
//!
//! **Position:** called by [`crate::cli::dispatch`]; hands the work to the `repository_relocation`
//! crate ([`repository_relocation::dry_run`], [`repository_relocation::apply`],
//! [`repository_relocation::verify`]).
//!
//! **Signals & state:** none.
//!
//! **Invariants:** `--dry-run` and `--apply` without `--manifest` are an error, never a run over
//! every manifest; `--verify` without one judges every committed stage manifest.

use super::cli::{RefactorCmd, RelocateArgs};
use anyhow::{Result, bail};
use repository_layout::prelude::find_repository_root;
use repository_relocation::prelude::*;

/// Run `cmd` and return its process exit code.
pub(crate) fn run(cmd: RefactorCmd) -> Result<u8> {
    match cmd {
        RefactorCmd::Relocate(args) => run_relocate(args),
    }
}

fn run_relocate(args: RelocateArgs) -> Result<u8> {
    let root = find_repository_root()?;
    if args.verify {
        return Ok(verify(&root, args.manifest.as_deref()));
    }
    let Some(manifest) = args.manifest else {
        bail!("--dry-run and --apply need --manifest <path.tsv>");
    };
    if args.apply {
        Ok(apply(&root, &manifest))
    } else {
        Ok(dry_run(&root, &manifest))
    }
}
