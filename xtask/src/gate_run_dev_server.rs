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

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use std::os::unix::process::CommandExt;

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
pub fn run_with_root(root: &Path, args: &[String]) -> Result<u8> {
    let real = root.join("scripts/mod/run-playtest-server.sh");

    // bash: `[ ! -x "$REAL" ]` → missing OR not executable → rc 3
    if !is_executable(&real) {
        eprintln!(
            "run-dev-server.sh: the real launcher is missing at {}",
            real.display()
        );
        eprintln!("  This shim starts nothing on its own — it never did.");
        return Ok(3);
    }

    // bash: `[ "$#" -eq 0 ]` → usage on stderr, rc 2
    if args.is_empty() {
        eprint!("{USAGE}");
        return Ok(2);
    }

    // bash: `exec "$REAL" "$@"` — replace this process image.
    let err = Command::new(&real).args(args).exec();
    // exec only returns on failure to launch
    eprintln!(
        "run-dev-server.sh: failed to exec {}: {err}",
        real.display()
    );
    Ok(127)
}

/// Mirror bash `-x`: true iff the path exists as a regular file with any execute bit.
fn is_executable(path: &Path) -> bool {
    match path.metadata() {
        Ok(m) if m.is_file() => m.permissions().mode() & 0o111 != 0,
        _ => false,
    }
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

    #[test]
    fn missing_playtest_is_rc3() {
        let root = throwaway("missing");
        let code = run_with_root(&root, &["--mission-id=x".into()]).unwrap();
        assert_eq!(code, 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn nonexec_playtest_is_rc3() {
        let root = throwaway("nonexec");
        let play = root.join("scripts/mod/run-playtest-server.sh");
        fs::write(&play, "#!/bin/bash\n").unwrap();
        let mut perms = fs::metadata(&play).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&play, perms).unwrap();
        let code = run_with_root(&root, &["--mission-id=x".into()]).unwrap();
        assert_eq!(code, 3);
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
