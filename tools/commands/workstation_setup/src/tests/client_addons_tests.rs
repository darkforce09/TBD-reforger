#![allow(
    unsafe_code,
    reason = "the tests set process variables under `tool_test_support::lock_env`; std offers no safe call that writes one"
)]

use super::*;
use crate::error::ResultExt;
use process_runner::PathGuard;
use repository_layout::workspace_folders::ENFUSION_MOD_DIR;
use std::fs;
use std::io;
use std::os::unix::fs as unix_fs;
use std::sync::MutexGuard;
use tool_test_support::lock_env;

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
    let code = run_with_root(&root, &home, &not_configured(&root)).unwrap();
    assert_eq!(code, 0);
    let link = home.join(".local/share/tbd-server-addons/tbd-framework");
    let meta = fs::symlink_metadata(&link).unwrap();
    assert!(meta.file_type().is_symlink());
    assert_eq!(
        fs::read_link(&link).unwrap(),
        root.join(FRAMEWORK_ADDON_DIR)
    );
}

#[test]
fn missing_framework_still_succeeds_dangling() {
    // Preserved oddity: bash `ln -sfn` does not require the target to exist.
    let _g = lock_env();
    let root = throwaway_root("miss-fw", false);
    let home = tempfile_dir("miss-fw-home");
    let code = run_with_root(&root, &home, &not_configured(&root)).unwrap();
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
    let code = run_with_root(&root, &home, &not_configured(&root)).unwrap();
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
    let code = run_with_root(&root, &home, &not_configured(&root)).unwrap();
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

    let saved: Vec<(&str, Option<std::ffi::OsString>)> = ["HOME", "TBD_SSH_HOST", "DEPLOY_ENV"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
    // SAFETY: under ENV_LOCK; restored below. The root is INJECTED (`run_in`), never reached
    // by chdir: `set_current_dir` is process-wide and made concurrent `find_repository_root`
    // callers in other test threads resolve this throwaway root (see `run_in`). No host is
    // configured, so the join hint asks no resolver.
    unsafe {
        std::env::set_var("HOME", &home);
        std::env::remove_var("TBD_SSH_HOST");
        std::env::remove_var("DEPLOY_ENV");
    }
    let code = run_in(&root).unwrap();
    for (key, value) in saved {
        match value {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
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
fn the_join_hint_names_the_host_its_address_or_the_setting_to_fill() {
    let resolved = DirectJoinTarget::Resolved {
        host: "staging.example".into(),
        address: Ipv4Addr::new(192, 0, 2, 10),
    };
    assert_eq!(
        direct_join_hint(&resolved),
        "Restart the game, then Direct Join → staging.example (192.0.2.10) port 2001"
    );
    let unresolved = DirectJoinTarget::Unresolved {
        host: "staging.example".into(),
        reason: "Name or service not known".into(),
    };
    assert_eq!(
        direct_join_hint(&unresolved),
        "Restart the game, then Direct Join → staging.example port 2001 \
         (no IPv4 address from here: Name or service not known)"
    );
    let not_configured = DirectJoinTarget::NotConfigured {
        path: PathBuf::from("/home/deploy/checkout/deploy/deploy.env"),
    };
    assert_eq!(
        direct_join_hint(&not_configured),
        "Restart the game, then Direct Join → the staging host, port 2001 \
         (set TBD_SSH_HOST in /home/deploy/checkout/deploy/deploy.env to print its address)"
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

/// The hint target of a checkout whose deploy settings name no host.
fn not_configured(root: &Path) -> DirectJoinTarget {
    DirectJoinTarget::NotConfigured {
        path: root.join(repository_layout::DEPLOY_ENV),
    }
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
    fs::write(root.join(".ai/tickets/ROOT"), "{}\n").unwrap();
    fs::create_dir_all(root.join(ENFUSION_MOD_DIR)).unwrap();
    if with_framework {
        fs::create_dir_all(root.join(FRAMEWORK_ADDON_DIR)).unwrap();
        fs::write(
            root.join(FRAMEWORK_ADDON_DIR).join("addon.gproj"),
            "gproj\n",
        )
        .unwrap();
    }
    root
}
