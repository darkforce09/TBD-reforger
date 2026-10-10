//! The shared `CARGO_TARGET_DIR` pin, the two checkout roots it is computed from, and the glibc
//! stamp guard.
//!
//! **Role:** answers where every build writes. `resolve_target_dir` is the shared cache pin
//! (`CARGO_TARGET_DIR` when set, else `primary_root``/target/<host|container>`, by
//! [`toolchain_environment`]); `dev_api_target_dir` is the
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
//!   `primary_root/target/<environment>`, shared by every worktree so parallel slices do not each
//!   cold-build the workspace; the development API's private directory is
//!   `cwd_root/target/<environment>/dev-api`, per checkout,
//!   because it starts a long-lived server that must not wait in the shared build-lock queue.
//!   Collapsing them either way is a silent regression, so they are two functions with two names.
//! - One folder per toolchain environment: a host build and a container build link against
//!   different glibcs, so they never share a target folder; [`abi_guard`] catches a hand-set
//!   `CARGO_TARGET_DIR` that crosses them.
//! - The pin is computed here and never moved into `.cargo/config.toml`: an `[env]` entry with
//!   `relative = true` resolves against the config file's own folder, which inside a linked
//!   worktree is that worktree, so every worktree would get its own cold `target/` and nothing would
//!   report it.

use std::path::{Path, PathBuf};

use process_runner::Run;
use repository_layout::build_output::{
    DEV_API_SUBFOLDER, ToolchainEnvironment, build_output_subfolder, toolchain_build_folder,
};

/// The checkout this process runs in. **Inside a worktree this is the worktree.** Used only for
/// the development API's private directory ([`dev_api_target_dir`]); never for the shared cache.
pub(crate) fn cwd_root() -> PathBuf {
    repository_root::find_repository_root().unwrap_or_else(|_| PathBuf::from("."))
}

/// The **primary** checkout: `git rev-parse --path-format=absolute --git-common-dir` with its
/// trailing `/.git` removed.
///
/// The same derivation as the wave gate steps' `main_root` fallback (`TTM_MAIN_ROOT` unset):
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

/// The toolchain environment this process builds in: the container when it runs inside the
/// development container, else the host.
pub(crate) fn toolchain_environment() -> ToolchainEnvironment {
    ToolchainEnvironment::from_container_flag(process_runner::host_execution::in_container())
}

/// `CARGO_TARGET_DIR` when set and non-empty, else the primary checkout's
/// `target/<host|container>`.
///
/// `env` is the caller's `$CARGO_TARGET_DIR`, threaded as a **parameter** rather than read from the
/// process environment, so a caller can ask "what would this be with the variable unset?" without
/// a `remove_var` (unsafe, global, and racy with any thread).
pub(crate) fn resolve_target_dir(env: Option<&str>) -> String {
    match env {
        // An operator or runner export wins: the ticket manager's runner hands its gate steps a
        // build folder, and a pin that overrode it would put every gate back in the shared cache.
        Some(v) if !v.is_empty() => v.to_string(),
        _ => toolchain_build_folder(&primary_root(), toolchain_environment())
            .display()
            .to_string(),
    }
}

/// `$CARGO_TARGET_DIR` from the environment, empty treated as unset.
pub(crate) fn env_pin() -> Option<String> {
    std::env::var("CARGO_TARGET_DIR")
        .ok()
        .filter(|s| !s.is_empty())
}

/// The development API's private `CARGO_TARGET_DIR`: `<this checkout>/target/<environment>/dev-api`.
pub(crate) fn dev_api_target_dir() -> PathBuf {
    build_output_subfolder(&cwd_root(), toolchain_environment(), DEV_API_SUBFOLDER)
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

/// `glibc<version>-<container|host>`. The container test is the host bridge's own
/// (`process_runner::host_execution::in_container`), and NOT `command -v distrobox-host-exec`,
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
    format!("glibc{glibc}-{}", toolchain_environment().folder_name())
}
