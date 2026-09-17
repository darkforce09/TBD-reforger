//! T-875 — port of `scripts/mod/setup-workbench-linux.sh` → `cargo xtask setup workbench`.
//!
//! Symlinks the Steam Arma Reforger `addons/data` tree to `$HOME/ArmaReforger-Base/data` so
//! Proton Workbench can browse a simple home path when "Locate base game" appears.
//!
//! Env:
//! - `HOME` — required (bash `set -u`); link root is `$HOME/ArmaReforger-Base`
//! - `STEAM_BASE` — optional override of the Steam common install dir (default under `$HOME`)
//!
//! Acceptance is bash/port stdout+stderr+rc on a clean tree and ≥2 broken arms (T-556 / T-853).
//! Throwaway `$HOME` / `STEAM_BASE` only — never clobber the operator's real Steam tree.
//!
//! Preserved oddities:
//! - Proton path uses `$(whoami)` (real login name), not a basename of `$HOME`.
//! - Success tip lines are byte-identical to the former script.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs as unix_fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Entry for `xtask setup workbench`.
pub fn run() -> Result<u8> {
    let home = std::env::var("HOME").context("HOME is unset (bash set -u would fail)")?;
    let steam_base = std::env::var("STEAM_BASE")
        .unwrap_or_else(|_| format!("{home}/.local/share/Steam/steamapps/common/Arma Reforger"));
    run_with_paths(Path::new(&home), Path::new(&steam_base))
}

/// Testable entry that does not read process env for paths.
pub fn run_with_paths(home: &Path, steam_base: &Path) -> Result<u8> {
    let src = steam_base.join("addons/data");
    let gproj = src.join("ArmaReforger.gproj");
    let link_root = home.join("ArmaReforger-Base");

    if !gproj.is_file() {
        eprintln!("Base game not found at:");
        eprintln!("  {}", gproj.display());
        eprintln!("Install Arma Reforger via Steam first.");
        return Ok(1);
    }

    fs::create_dir_all(&link_root).with_context(|| format!("mkdir -p {}", link_root.display()))?;

    let link = link_root.join("data");
    force_symlink(&src, &link)?;

    let user = whoami_name()?;
    println!("Symlink ready:");
    println!("  Linux:  {}/ArmaReforger.gproj", link.display());
    println!("  Proton: Z:\\home\\{user}\\ArmaReforger-Base\\data\\ArmaReforger.gproj");
    println!();
    println!(
        "In Workbench 'Locate base game', browse to ArmaReforger.gproj at one of the paths above."
    );
    println!(
        "Tip: Launch Arma Reforger (the game) once, quit, then open Workbench — auto-detect may work after that."
    );

    Ok(0)
}

/// `ln -sfn "$target" "$link"` — replace existing file/symlink at `link`.
fn force_symlink(target: &Path, link: &Path) -> Result<()> {
    match fs::symlink_metadata(link) {
        Ok(meta) => {
            if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
                fs::remove_dir_all(link).with_context(|| format!("rm -rf {}", link.display()))?;
            } else {
                fs::remove_file(link).with_context(|| format!("rm -f {}", link.display()))?;
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

/// Match bash `$(whoami)` — login name, independent of `$HOME`.
fn whoami_name() -> Result<String> {
    let out = Command::new("whoami").output().context("whoami")?;
    if !out.status.success() {
        let mut err = io::stderr().lock();
        let _ = err.write_all(&out.stderr);
        bail!("whoami exited {}", out.status);
    }
    let name = String::from_utf8(out.stdout).context("whoami stdout utf-8")?;
    Ok(name.trim_end_matches(['\n', '\r']).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env::{PathGuard, lock_env};
    use std::path::PathBuf;
    use std::sync::MutexGuard;

    #[test]
    fn missing_gproj_exits_1() {
        let _g = lock_env();
        let home = tempfile_dir("no-gproj");
        let steam = home.join("steam-base");
        fs::create_dir_all(steam.join("addons/data")).unwrap();
        // addons/data exists but no ArmaReforger.gproj
        let code = run_with_paths(&home, &steam).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn missing_steam_tree_exits_1() {
        let _g = lock_env();
        let home = tempfile_dir("no-steam");
        let steam = home.join("no-such-steam");
        let code = run_with_paths(&home, &steam).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn clean_tree_symlinks_and_prints() {
        let _g = lock_env();
        let home = tempfile_dir("clean");
        let steam = home.join("steam-base");
        let data = steam.join("addons/data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("ArmaReforger.gproj"), "gproj\n").unwrap();

        let code = run_with_paths(&home, &steam).unwrap();
        assert_eq!(code, 0);

        let link = home.join("ArmaReforger-Base/data");
        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink());
        let target = fs::read_link(&link).unwrap();
        assert_eq!(target, data);
        assert!(link.join("ArmaReforger.gproj").is_file());
    }

    #[test]
    fn run_reads_home_and_steam_base_env() {
        let _g: MutexGuard<'_, ()> = lock_env();
        let home = tempfile_dir("env-run");
        let steam = home.join("steam-base");
        let data = steam.join("addons/data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("ArmaReforger.gproj"), "gproj\n").unwrap();

        // Keep system bins on PATH (wave-224 flake: never stub-only PATH).
        let _path = PathGuard::prepend_dir(Path::new("/usr/bin"));

        let prev_home = std::env::var_os("HOME");
        let prev_steam = std::env::var_os("STEAM_BASE");
        // SAFETY: under ENV_LOCK; restored below.
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("STEAM_BASE", &steam);
        }
        let code = run().unwrap();
        match prev_home {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        match prev_steam {
            Some(v) => unsafe { std::env::set_var("STEAM_BASE", v) },
            None => unsafe { std::env::remove_var("STEAM_BASE") },
        }
        assert_eq!(code, 0);
        assert!(
            home.join("ArmaReforger-Base/data/ArmaReforger.gproj")
                .is_file()
        );
    }

    #[test]
    fn relink_replaces_existing_symlink() {
        let _g = lock_env();
        let home = tempfile_dir("relink");
        let steam_a = home.join("steam-a");
        let steam_b = home.join("steam-b");
        for steam in [&steam_a, &steam_b] {
            let data = steam.join("addons/data");
            fs::create_dir_all(&data).unwrap();
            fs::write(data.join("ArmaReforger.gproj"), "gproj\n").unwrap();
        }
        assert_eq!(run_with_paths(&home, &steam_a).unwrap(), 0);
        assert_eq!(run_with_paths(&home, &steam_b).unwrap(), 0);
        let link = home.join("ArmaReforger-Base/data");
        assert_eq!(fs::read_link(&link).unwrap(), steam_b.join("addons/data"));
    }

    fn tempfile_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "t875-{tag}-{}-{}",
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
}
