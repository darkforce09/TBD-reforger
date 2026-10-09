//! The shared `CARGO_TARGET_DIR` pin, the two checkout roots it is computed from, and the glibc
//! stamp guard.
//!
//! **Role:** answers where every build writes. `resolve_target_dir` is the shared cache pin
//! (`CARGO_TARGET_DIR` when set, else `primary_root``/target`); `dev_api_target_dir` is the
//! development API's private folder under `cwd_root`; [`abi_guard`] refuses a target directory
//! that another glibc built.
//! **Position:** read by the `mk` recipes ([`crate::build_lane::recipes`]), the xtask wave
//! driver's flush (through [`abi_guard`]) and `mk reclaim-target-ci`. The subfolder names and their formula are `repository_layout::build_output`'s.
//! **Signals & state:** none held; `primary_root` asks `git`, and [`abi_guard`] writes one
//! stamp file per target directory.
//! **Invariants:**
//! - Two roots, never one. `primary_root` (the primary checkout, from `git rev-parse
//!   --git-common-dir`) and `cwd_root` (this checkout) are the same folder in the primary
//!   checkout and different inside a linked worktree. The shared warm cache is
//!   `primary_root/target`, shared by every worktree so parallel slices do not each cold-build the
//!   workspace; the development API's private directory is `cwd_root/target/dev-api`, per checkout,
//!   because it starts a long-lived server that must not wait in the shared build-lock queue.
//!   Collapsing them either way is a silent regression, so they are two functions with two names.
//! - The pin is computed here and never moved into `.cargo/config.toml`: an `[env]` entry with
//!   `relative = true` resolves against the config file's own folder, which inside a linked
//!   worktree is that worktree, so every worktree would get its own cold `target/` and nothing would
//!   report it.

use std::path::{Path, PathBuf};

use process_runner::Run;
use repository_layout::build_output::{DEV_API_SUBFOLDER, build_output_subfolder};

/// The checkout this process runs in. **Inside a worktree this is the worktree.** Used only for
/// the development API's private directory ([`dev_api_target_dir`]); never for the shared cache.
pub(crate) fn cwd_root() -> PathBuf {
    repository_root::find_repository_root().unwrap_or_else(|_| PathBuf::from("."))
}

/// The **primary** checkout: `git rev-parse --path-format=absolute --git-common-dir` with its
/// trailing `/.git` removed.
///
/// The same derivation as the xtask wave driver's `main_root`:
/// one formula, not two. A git that cannot answer falls back to this checkout, which is at worst a
/// cold build and never a write to `/`.
pub(crate) fn primary_root() -> PathBuf {
    let common = Run::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()
        .filter(|o| o.code == 0)
        .map(|o| o.stdout.trim().to_string())
        .filter(|s| !s.is_empty());
    match common {
        Some(g) => Path::new(&g)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(cwd_root),
        None => cwd_root(),
    }
}

/// `CARGO_TARGET_DIR` when set and non-empty, else the primary checkout's `target/`.
///
/// `env` is the caller's `$CARGO_TARGET_DIR`, threaded as a **parameter** rather than read from the
/// process environment, so a caller can ask "what would this be with the variable unset?" without
/// a `remove_var` (unsafe, global, and racy with any thread).
pub(crate) fn resolve_target_dir(env: Option<&str>) -> String {
    match env {
        // An operator or driver export wins: the wave driver hands its gate steps a private
        // directory, and a pin that overrode it would put every gate back in the shared cache.
        Some(v) if !v.is_empty() => v.to_string(),
        _ => primary_root().join("target").display().to_string(),
    }
}

/// `$CARGO_TARGET_DIR` from the environment, empty treated as unset.
pub(crate) fn env_pin() -> Option<String> {
    std::env::var("CARGO_TARGET_DIR")
        .ok()
        .filter(|s| !s.is_empty())
}

/// The development API's private `CARGO_TARGET_DIR`: `<this checkout>/target/dev-api`.
pub(crate) fn dev_api_target_dir() -> PathBuf {
    build_output_subfolder(&cwd_root(), DEV_API_SUBFOLDER)
}

/// The ABI allowed to write into a given target directory: stamped on first use, enforced after.
///
/// Two glibcs sharing one `CARGO_TARGET_DIR` produce `GLIBC_2.xx not found` at run time, a link
/// error that reads like a broken checkout; this turns it into a named refusal at the boundary. An
/// unreadable or unwritable stamp is **not** a failure: the guard catches one specific collision,
/// and a guard that blocked builds over a permissions quirk would simply be disabled.
pub fn abi_guard(dir: &Path) -> std::result::Result<(), String> {
    let want = abi_id();
    let stamp = dir.join(".tbd-build-abi");
    if let Ok(found) = std::fs::read_to_string(&stamp) {
        let found = found.trim();
        if !found.is_empty() && found != want {
            return Err(format!(
                "REFUSING: {} was built by '{found}', this is '{want}'.\n      \
                 Two glibcs sharing one CARGO_TARGET_DIR produce `GLIBC_2.xx not found` at run \
                 time, which reads like a broken checkout.\n      \
                 Set CARGO_TARGET_DIR to a directory of your own, or delete {}.",
                dir.display(),
                stamp.display()
            ));
        }
        return Ok(());
    }
    if std::fs::create_dir_all(dir).is_ok() {
        let _ = std::fs::write(&stamp, format!("{want}\n"));
    }
    Ok(())
}

/// `glibc<version>-<container|host>`. The container test is distrobox's own (`/run/.containerenv`
/// or `/.dockerenv`), the same one the host bridge uses, and NOT `command -v distrobox-host-exec`,
/// which is true on both sides of the bridge.
// `gnu_get_libc_version` is FFI, which `unsafe_code` cannot tell from unsound code; the call is
// sound (see the SAFETY note) and std has no safe way to ask the glibc version.
#[allow(unsafe_code)]
pub(crate) fn abi_id() -> String {
    // SAFETY: `gnu_get_libc_version` returns a pointer to a static NUL-terminated string in libc;
    // it takes no arguments, allocates nothing, and the result outlives this call.
    let glibc = unsafe {
        let p = libc::gnu_get_libc_version();
        if p.is_null() {
            "unknown".to_string()
        } else {
            std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    };
    let where_ = if Path::new("/run/.containerenv").exists() || Path::new("/.dockerenv").exists() {
        "container"
    } else {
        "host"
    };
    format!("glibc{glibc}-{where_}")
}
