//! `cargo xtask mod dev-server` — the argument gate in front of the playtest launcher.
//!
//! It starts nothing itself: with arguments it hands them to
//! [`crate::commands::mod_ops::playtest_server`], and with none it prints [`USAGE`] and exits 2,
//! because a dedicated server told no mission boots into LOADING and stays there looking healthy.
//!
//! Exit codes:
//! - **2** — no arguments (usage naming the playtest command and its two decisive flags)
//! - otherwise — whatever the playtest launcher returns
//!
//! Nothing here is silenced: every refusal is loud on stderr.

use std::path::Path;

use anyhow::Result;

use crate::core::repository_root::find_repo_root;

/// What a bare `cargo xtask mod dev-server` prints before exiting 2.
const USAGE: &str = "\
cargo xtask mod dev-server starts nothing on its own — it hands its arguments to
cargo xtask mod playtest, which has to be told WHICH mission to serve.

  cargo xtask mod playtest --mission-id=<id> [--admin=<identityId>]

  --mission-id   the mission the mod loads. Without it the stage machine never leaves
                 LOADING and the server looks healthy while being unplayable.
  --admin        your identityId (UUID) or 17-digit SteamID. Without it every '#tbd'
                 command answers \"TBD: admin only.\" and no admin command can be tested.

  cargo xtask mod playtest --help    for the rest
  docs/mod/STAGING-SERVER.md         for what the second client needs

Offline? Add --mission-file=contracts_v2/fixtures/missions/valid/bridgehead-at-levie.json
to serve a golden from disk with no API running.\n";

/// Entry for `xtask mod dev-server [args…]`.
pub fn run(args: &[String]) -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root, args)
}

/// Testable entry that does not walk for the repo root.
pub fn run_with_root(_root: &Path, args: &[String]) -> Result<u8> {
    // No arguments is the one refusal this gate owns: usage on stderr, rc 2.
    if args.is_empty() {
        eprint!("{USAGE}");
        return Ok(2);
    }

    // The launcher is [`crate::commands::mod_ops::playtest_server`], linked into this binary, so
    // there is no state in which it is absent while this line runs and no "launcher missing"
    // outcome to report — the type system discharges that question.
    crate::commands::mod_ops::playtest_server::run(args)
}

#[cfg(test)]
#[path = "tests/development_server/tests.rs"]
mod tests;
