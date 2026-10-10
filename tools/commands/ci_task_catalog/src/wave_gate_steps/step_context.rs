//! What every gate step resolves at entry: the checkouts, the build folders and the host bridge.
//!
//! **Role:** builds the step [`Ctx`] from the environment the ticket manager's runner exports
//! (`TTM_ROOT`, `TTM_MAIN_ROOT`, `CARGO_TARGET_DIR`, `TTM_GATE_LOCK`), falling back to git when a
//! step runs by hand; wraps the git queries the steps share and the porcelain read that never
//! reads a failure as a clean tree; answers whether a gate holds the gate lock.
//! **Position:** used by every module of `wave_gate_steps`.
//! **Signals & state:** [`Ctx::from_environment`] moves the process into the checkout root, so the
//! relative paths the steps use (`Cargo.toml`, member folders) mean the same thing everywhere.
//! **Invariants:** a git query that fails yields `None` or an empty string, never a fabricated
//! answer; the lock probe reports "held" only when a non-blocking `flock` on the lock file is
//! refused, so an unlocked file can never read as held.

use std::fs::{OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};

use repository_layout::build_output::{
    self, ToolchainEnvironment, build_output_subfolder, toolchain_build_folder,
};

use super::host;

/// Everything a gate step resolves once.
pub(crate) struct Ctx {
    /// This checkout (a slice worktree or the main checkout).
    pub root: PathBuf,
    /// The main checkout, shared by every worktree.
    pub main_root: PathBuf,
    /// The shared build folder every step's `cargo` defaults to.
    pub cargo_target_dir: String,
    pub gate_trunk_target: String,
    pub gate_trunk_dist: String,
    pub gate_check_target: String,
    pub gate_schema_target: String,
    pub host: host::Host,
}

fn env_nonempty(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.is_empty())
}

impl Ctx {
    /// Resolve the context and enter the checkout root.
    pub(crate) fn from_environment() -> std::io::Result<Ctx> {
        let cwd = std::env::current_dir()?;
        let root = env_nonempty("TTM_ROOT")
            .map(PathBuf::from)
            .or_else(|| git_stdout(&["rev-parse", "--show-toplevel"]).map(PathBuf::from))
            .unwrap_or(cwd);
        std::env::set_current_dir(&root)?;
        let main_root = env_nonempty("TTM_MAIN_ROOT")
            .map(PathBuf::from)
            .or_else(|| {
                git_stdout(&["rev-parse", "--path-format=absolute", "--git-common-dir"])
                    .and_then(|common| Path::new(&common).parent().map(Path::to_path_buf))
            })
            .unwrap_or_else(|| root.clone());
        let cargo_target_dir = env_nonempty("CARGO_TARGET_DIR").unwrap_or_else(|| {
            toolchain_build_folder(&main_root, toolchain_environment())
                .display()
                .to_string()
        });
        let folder = |var: &str, sub: &str| {
            env_nonempty(var).unwrap_or_else(|| gate_folder(&main_root, sub))
        };
        let timeout = env_nonempty("TTM_GATE_TIMEOUT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1200);
        Ok(Ctx {
            gate_trunk_target: folder("TBD_GATE_TRUNK_TARGET", build_output::GATE_TRUNK_SUBFOLDER),
            gate_trunk_dist: folder(
                "TBD_GATE_TRUNK_DIST",
                build_output::GATE_FRONTEND_DIST_SUBFOLDER,
            ),
            gate_check_target: folder("TBD_GATE_CHECK_TARGET", build_output::GATE_CHECK_SUBFOLDER),
            gate_schema_target: folder(
                "TBD_GATE_SCHEMA_TARGET",
                build_output::GATE_SCHEMA_SUBFOLDER,
            ),
            host: host::Host::detect(timeout),
            cargo_target_dir,
            main_root,
            root,
        })
    }
}

/// The toolchain environment this step runs in: the container, or the host.
pub(crate) fn toolchain_environment() -> ToolchainEnvironment {
    ToolchainEnvironment::from_container_flag(process_runner::host_execution::in_container())
}

/// `<main checkout>/target/<environment>/<subfolder>`: a gate step's private build folder.
pub(crate) fn gate_folder(main_root: &Path, subfolder: &str) -> String {
    build_output_subfolder(main_root, toolchain_environment(), subfolder)
        .display()
        .to_string()
}

/// Flush stdout before stderr is written or a child inherits stdout.
pub(crate) fn flush() {
    let _ = std::io::stdout().flush();
}

/// `git … 2>/dev/null` capturing trimmed stdout, `None` on any failure.
pub(crate) fn git_stdout(args: &[&str]) -> Option<String> {
    let out = process_runner::Run::new("git").args(args).output().ok()?;
    if out.code != 0 {
        return None;
    }
    Some(out.stdout.trim_end_matches('\n').to_string())
}

/// As [`git_stdout`], keeping the output whatever git's exit code.
pub(crate) fn git_stdout_lossy(args: &[&str]) -> String {
    match process_runner::Run::new("git").args(args).output() {
        Ok(out) => out.stdout.trim_end_matches('\n').to_string(),
        Err(_) => String::new(),
    }
}

/// The `-c filter.lfs.*` flags that keep a read-only git call working without git-lfs.
pub(crate) const LFS_NEUTRAL: [&str; 8] = [
    "-c",
    "filter.lfs.process=",
    "-c",
    "filter.lfs.clean=cat",
    "-c",
    "filter.lfs.smudge=cat",
    "-c",
    "filter.lfs.required=false",
];

/// Working-tree porcelain paths with the LFS filters neutralised; `Err(rc)` when git failed,
/// never an empty list standing in for a failure.
pub(crate) fn git_porcelain_paths() -> Result<Vec<String>, i32> {
    let out = process_runner::Run::new("git")
        .args(LFS_NEUTRAL)
        .args(["status", "--porcelain"])
        .output();
    let (stdout, rc) = match out {
        Ok(o) => (o.stdout, o.code),
        Err(cause) => (String::new(), host::status_code(Err(cause))),
    };
    if rc != 0 {
        eprintln!(
            "gate: git status --porcelain failed (rc={rc}) — refusing a silent empty change list"
        );
        return Err(rc);
    }
    Ok(stdout
        .lines()
        .map(|l| l.get(3..).unwrap_or("").to_string())
        .collect())
}

/// Whether a gate holds the gate lock (`TTM_GATE_LOCK`), and whether the operator let the gate
/// run unserialised (`TTM_GATE_UNSERIALISED=1`).
pub(crate) struct GateState {
    held: bool,
    unserialised: bool,
}

impl GateState {
    /// Probe the lock the runner exported: held when a non-blocking lock attempt is refused.
    pub(crate) fn probe() -> GateState {
        let held = env_nonempty("TTM_GATE_LOCK").is_some_and(|path| {
            OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&path)
                .is_ok_and(|file| matches!(file.try_lock(), Err(TryLockError::WouldBlock)))
        });
        GateState {
            held,
            unserialised: env_nonempty("TTM_GATE_UNSERIALISED").as_deref() == Some("1"),
        }
    }

    pub(crate) fn held(&self) -> bool {
        self.held
    }

    pub(crate) fn unserialised(&self) -> bool {
        self.unserialised
    }
}
