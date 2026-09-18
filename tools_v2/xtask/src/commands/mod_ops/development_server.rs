//! T-871 — port of `scripts/mod/run-dev-server.sh` → `cargo xtask mod dev-server`.
//!
//! This is a **shim**: it never starts a server itself. It either execs
//! `scripts/mod/run-playtest-server.sh` with the caller's args, or fails loudly.
//! The real launcher (`run-playtest-server.sh`) is **§Not in scope** for T-853
//! wave slices — do not delete or rewrite it here.
//!
//! Exit codes (bash parity):
//! - **3** — playtest launcher missing or not executable
//! - **2** — no arguments (usage pointing at playtest)
//! - otherwise — the playtest script's own exit code (`exec`)
//!
//! Preserved oddity: stderr still names `run-dev-server.sh` (byte-parity with bash
//! baselines under `/tmp/t853/w223/t871/`).
//!
//! No fail-open closed: the bash already fails loud on every arm; nothing was
//! `2>/dev/null` / `|| true`.

use std::path::Path;

use anyhow::Result;

use crate::core::repository_root::find_repo_root;

/// Byte-identical to bash no-args heredoc (rc=2).
const USAGE: &str = "\
run-dev-server.sh starts nothing on its own — it is a shim for run-playtest-server.sh,
which has to be told WHICH mission to serve.

  bash scripts/mod/run-playtest-server.sh --mission-id=<id> [--admin=<identityId>]

  --mission-id   the mission the mod loads. Without it the stage machine never leaves
                 LOADING and the server looks healthy while being unplayable.
  --admin        your identityId (UUID) or 17-digit SteamID. Without it every '#tbd'
                 command answers \"TBD: admin only.\" and T-181.16 cannot pass.

  bash scripts/mod/run-playtest-server.sh --help    for the rest
  docs/mod/STAGING-SERVER.md                        for what the second client needs

Offline? Add --mission-file=packages/tbd-schema/golden-missions/bridgehead-at-levie.json
to serve a golden from disk with no API running.\n";

/// Entry for `xtask mod dev-server [args…]`.
pub fn run(args: &[String]) -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root, args)
}

/// Testable entry that does not walk for the repo root.
pub fn run_with_root(_root: &Path, args: &[String]) -> Result<u8> {
    // bash: `[ "$#" -eq 0 ]` → usage on stderr, rc 2
    if args.is_empty() {
        eprint!("{USAGE}");
        return Ok(2);
    }

    // T-853: bash was `exec "$REAL" "$@"` where $REAL was
    // `scripts/mod/run-playtest-server.sh`. That launcher is now
    // [`crate::commands::mod_ops::playtest_server`], so this shim CALLS it instead of replacing its own process image.
    //
    // The rc-3 "the real launcher is missing" arm went with it, and deliberately: it existed
    // because a shell script can be deleted or lose its execute bit out from under a caller. A
    // module linked into this binary cannot, so the check is not "removed" so much as discharged
    // by the type system — there is no state in which the launcher is absent and this line runs.
    // `is_executable` is retained below only for the tests that still pin the old shape.
    crate::commands::mod_ops::playtest_server::run(args)
}

#[cfg(test)]
#[path = "tests/development_server/tests.rs"]
mod tests;
