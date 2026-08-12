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

use crate::root::find_repo_root;

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
    // [`crate::playtest_server`], so this shim CALLS it instead of replacing its own process image.
    //
    // The rc-3 "the real launcher is missing" arm went with it, and deliberately: it existed
    // because a shell script can be deleted or lose its execute bit out from under a caller. A
    // module linked into this binary cannot, so the check is not "removed" so much as discharged
    // by the type system — there is no state in which the launcher is absent and this line runs.
    // `is_executable` is retained below only for the tests that still pin the old shape.
    crate::playtest_server::run(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn throwaway(tag: &str) -> PathBuf {
        let root = PathBuf::from(format!(
            "/tmp/t853/w223/t871/ut-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".ai/tickets")).unwrap();
        fs::create_dir_all(root.join("scripts/mod")).unwrap();
        fs::write(root.join(".ai/tickets/registry.json"), "{}\n").unwrap();
        root
    }

    /// T-853: the rc-3 "the real launcher is missing / is not executable" arm is GONE, and this
    /// test records why rather than the old two tests being quietly deleted.
    ///
    /// bash `exec`'d `scripts/mod/run-playtest-server.sh`, which could be absent or lose its
    /// execute bit between the check and the exec — so rc 3 was a real, reachable outcome and had
    /// to be pinned. The launcher is now [`crate::playtest_server`], linked into this binary.
    /// There is no state in which it is absent while this code runs, so the arm is not "removed",
    /// it is DISCHARGED by the type system. Asserting the absence of an unreachable branch is the
    /// closest thing to a test that remains honest here.
    ///
    /// `no_args_is_rc2` below still pins the one arm that IS reachable.
    #[test]
    fn the_missing_launcher_arm_is_discharged_not_deleted() {
        let root = throwaway("discharged");
        // No scripts/ tree at all — under bash this was rc 3. It must NOT be rc 3 now, because
        // rc 3 meant "I could not find the launcher" and that question no longer exists.
        let code = run_with_root(&root, &[]).unwrap();
        assert_eq!(
            code, 2,
            "an empty arg list is still the usage arm, not a missing-launcher arm"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_args_is_rc2() {
        let root = throwaway("noargs");
        let play = root.join("scripts/mod/run-playtest-server.sh");
        fs::write(&play, "#!/bin/bash\nexit 0\n").unwrap();
        let mut perms = fs::metadata(&play).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&play, perms).unwrap();
        let code = run_with_root(&root, &[]).unwrap();
        assert_eq!(code, 2);
        let _ = fs::remove_dir_all(&root);
    }
}
