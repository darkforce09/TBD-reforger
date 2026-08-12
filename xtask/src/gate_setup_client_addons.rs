//! T-878 — port of `scripts/mod/setup-client-addons.sh` → `cargo xtask setup client-addons`.
//!
//! Path pins mirror `scripts/mod/lib/paths.sh` (do **not** delete paths.sh — T-879):
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`. Staging lives at `$HOME/.local/share/tbd-server-addons`.
//!
//! Symlinks `$MOD_ROOT/tbd-framework` into the client addon staging dir and prints Steam launch
//! options. Acceptance is bash/port stdout+stderr+rc on a clean tree and ≥2 broken arms
//! (T-556 / T-853). Throwaway `$HOME` only — never clobber the operator's real addon staging.
//!
//! Preserved oddities:
//! - `ln -sfn` succeeds even when `$MOD_ROOT/tbd-framework` is missing (dangling symlink) —
//!   same as bash; not a red arm.
//! - Success banner (staging path + Steam launch options + Direct Join tip) is byte-identical
//!   to the former script.
//!
//! Fail-opens closed vs bash: none — the script had no `2>/dev/null` / `|| true` on the
//! mkdir/ln path (`set -euo pipefail`).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::root::find_repo_root;

/// Paths mirroring `scripts/mod/lib/paths.sh` for an already-resolved monorepo root.
struct Paths {
    mod_root: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mod_root: root.join("apps/mod"),
        }
    }
}

/// Entry for `xtask setup client-addons`.
pub fn run() -> Result<u8> {
    let root = find_repo_root()?;
    let home = std::env::var("HOME").context("HOME is unset (bash set -u would fail)")?;
    run_with_root(&root, Path::new(&home))
}

/// Testable entry that does not walk for the repo root or read `$HOME` from the process.
pub fn run_with_root(root: &Path, home: &Path) -> Result<u8> {
    let paths = Paths::from_root(root);
    let staging = home.join(".local/share/tbd-server-addons");
    let framework = paths.mod_root.join("tbd-framework");
    let link = staging.join("tbd-framework");

    // bash: `mkdir -p "$STAGING"` — shell out so broken-arm stderr matches GNU mkdir.
    let mkdir_status = Command::new("mkdir")
        .arg("-p")
        .arg(&staging)
        .status()
        .context("mkdir -p")?;
    if !mkdir_status.success() {
        return Ok(mkdir_status.code().unwrap_or(1) as u8);
    }

    // bash: `ln -sfn "$MOD_ROOT/tbd-framework" "$STAGING/tbd-framework"`
    // Shell out for GNU ln stderr parity on permission / nesting arms.
    let ln_status = Command::new("ln")
        .arg("-sfn")
        .arg(&framework)
        .arg(&link)
        .status()
        .context("ln -sfn")?;
    if !ln_status.success() {
        return Ok(ln_status.code().unwrap_or(1) as u8);
    }

    println!("Client addon staging: {}", link.display());
    println!();
    println!("Steam → Arma Reforger → Properties → Launch Options:");
    println!(
        "  -addonsDir \"{}\" -addons B2C3D4E5F6A78901",
        staging.display()
    );
    println!();
    println!("Restart the game, then Direct Join → 192.168.0.140 port 2001");

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env::{PathGuard, lock_env};
    use anyhow::Context;
    use std::fs;
    use std::io;
    use std::os::unix::fs as unix_fs;
    use std::sync::MutexGuard;

    /// Pure-Rust `ln -sfn` used only by unit tests that assert link shape without spawning.
    fn force_symlink_like_sfn(target: &Path, link: &Path) -> Result<()> {
        match fs::symlink_metadata(link) {
            Ok(meta) => {
                if meta.file_type().is_symlink() || meta.is_file() {
                    fs::remove_file(link).with_context(|| format!("rm -f {}", link.display()))?;
                } else if meta.is_dir() {
                    // GNU `ln -sfn` without `-T` nests inside a real directory — not an error.
                    let nested = link.join(
                        target
                            .file_name()
                            .context("ln -sfn target has no file_name")?,
                    );
                    return force_symlink_like_sfn(target, &nested);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(e).with_context(|| format!("stat {}", link.display()));
            }
        }
        unix_fs::symlink(target, link)
            .with_context(|| format!("ln -sfn {} {}", target.display(), link.display()))?;
        Ok(())
    }

    #[test]
    fn clean_tree_symlinks_and_prints() {
        let _g = lock_env();
        let root = throwaway_root("clean", true);
        let home = tempfile_dir("clean-home");
        let code = run_with_root(&root, &home).unwrap();
        assert_eq!(code, 0);
        let link = home.join(".local/share/tbd-server-addons/tbd-framework");
        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink());
        assert_eq!(
            fs::read_link(&link).unwrap(),
            root.join("apps/mod/tbd-framework")
        );
    }

    #[test]
    fn missing_framework_still_succeeds_dangling() {
        // Preserved oddity: bash `ln -sfn` does not require the target to exist.
        let _g = lock_env();
        let root = throwaway_root("miss-fw", false);
        let home = tempfile_dir("miss-fw-home");
        let code = run_with_root(&root, &home).unwrap();
        assert_eq!(code, 0);
        let link = home.join(".local/share/tbd-server-addons/tbd-framework");
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(!link.exists()); // dangling
    }

    #[test]
    fn staging_is_file_exits_1() {
        let _g = lock_env();
        let root = throwaway_root("staging-file", true);
        let home = tempfile_dir("staging-file-home");
        fs::create_dir_all(home.join(".local/share")).unwrap();
        fs::write(home.join(".local/share/tbd-server-addons"), "x\n").unwrap();
        let code = run_with_root(&root, &home).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn staging_not_writable_ln_exits_1() {
        let _g = lock_env();
        let root = throwaway_root("nowrite", true);
        let home = tempfile_dir("nowrite-home");
        let staging = home.join(".local/share/tbd-server-addons");
        fs::create_dir_all(&staging).unwrap();
        let mut perms = fs::metadata(&staging).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o555);
        fs::set_permissions(&staging, perms).unwrap();
        let code = run_with_root(&root, &home).unwrap();
        // restore so Temp cleanup can remove
        let mut perms = fs::metadata(&staging).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&staging, perms).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn run_reads_home_env() {
        let _g: MutexGuard<'_, ()> = lock_env();
        let root = throwaway_root("env-run", true);
        let home = tempfile_dir("env-run-home");
        // Keep system bins on PATH (never stub-only PATH).
        let _path = PathGuard::prepend_dir(Path::new("/usr/bin"));

        let prev_home = std::env::var_os("HOME");
        let prev_cwd = std::env::current_dir().unwrap();
        // SAFETY: under ENV_LOCK; restored below.
        unsafe {
            std::env::set_var("HOME", &home);
        }
        std::env::set_current_dir(&root).unwrap();
        let code = run().unwrap();
        std::env::set_current_dir(&prev_cwd).unwrap();
        match prev_home {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        assert_eq!(code, 0);
        assert!(
            home.join(".local/share/tbd-server-addons/tbd-framework")
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn force_symlink_like_sfn_replaces_symlink() {
        let dir = tempfile_dir("sfn");
        let target_a = dir.join("a");
        let target_b = dir.join("b");
        fs::write(&target_a, "a\n").unwrap();
        fs::write(&target_b, "b\n").unwrap();
        let link = dir.join("link");
        force_symlink_like_sfn(&target_a, &link).unwrap();
        force_symlink_like_sfn(&target_b, &link).unwrap();
        assert_eq!(fs::read_link(&link).unwrap(), target_b);
    }

    fn tempfile_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "t878-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn throwaway_root(tag: &str, with_framework: bool) -> PathBuf {
        let root = tempfile_dir(tag);
        fs::create_dir_all(root.join(".ai/tickets")).unwrap();
        fs::write(root.join(".ai/tickets/registry.json"), "{}\n").unwrap();
        fs::create_dir_all(root.join("apps/mod")).unwrap();
        if with_framework {
            fs::create_dir_all(root.join("apps/mod/tbd-framework")).unwrap();
            fs::write(root.join("apps/mod/tbd-framework/addon.gproj"), "gproj\n").unwrap();
        }
        root
    }
}
