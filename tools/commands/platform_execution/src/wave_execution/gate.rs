//! The two gate drivers: the cheap per-slice gate and the full wave gate.
//!
//! **Role:** holds the step `Runner`, the ten `cargo xtask verify` steps both gates share
//! (`VERIFY_STEPS`), the derivation of the tool crates the wave gate lints and the native clippy
//! command line, and re-exports `gate_slice` and `cmd_gate`.
//!
//! **Position:** reached through the wave command table (`wave gate [--slice <id>] [<base>]`); the
//! drivers live in `gate/checkrun.rs` and `gate/gate_dispatch.rs`. Neither runs Chrome or the
//! editor suite; those run through `cargo xtask mk leptos-gates`, a pre-close step of the editor
//! factory (`documentation/runbooks/factory_waves/README.md` §5). `cargo xtask ci ci-local` is not
//! used: it takes far longer than a gate.
//!
//! **Signals & state:** the runner's `fail` flag accumulates across steps; every step's stdout and
//! stderr are captured together.
//!
//! **Invariants:** the gates are not fail-fast — every step runs and the verdict is red when any
//! step was; a failing step prints `FAIL` and its last 15 captured lines indented six spaces; the
//! wave gate's clippy steps together name every workspace member, derived from the root manifest
//! (the tool lint every member under `tools/`, the frontend steps the frontend family, and
//! `gate/clippy_package_sets.rs` the wasm32 members and every other application and crate), and a
//! workspace without xtask, developer_tools or api in its lane is red, never a smaller lint.

use super::{
    Ctx, base, changed, db, git_stdout_lossy, host, lock::GateState, migrate, schema, touch, trunk,
    verdict,
};
use crate::wave_execution::{wprint, wprintln};

/// One step. `f` returns the step's rc; stdout and stderr are captured together.
struct Runner {
    fail: bool,
    /// The wave gate's runner carries a distinct arm for a timeout's exit 124 so the most
    /// expensive step's deadline is not relabelled as a code error. The slice gate sets none.
    timeout_arm: Option<u64>,
}

impl Runner {
    fn run(&mut self, label: &str, f: impl FnOnce() -> i32) {
        wprint!("  {label:<28} ");
        let (out, rc) = super::capture_step(f);
        if rc == 0 {
            wprintln!("PASS");
            return;
        }
        if let Some(secs) = self.timeout_arm
            && rc == 124
        {
            wprintln!("FAIL (TIMEOUT after {secs}s)");
            self.fail = true;
            return;
        }
        wprintln!("FAIL");
        // The last 15 captured lines, indented six spaces.
        let lines: Vec<&str> = out.lines().collect();
        for l in lines.iter().skip(lines.len().saturating_sub(15)) {
            wprintln!("      {l}");
        }
        self.fail = true;
    }
}

/// The ten `xtask verify` Class-R steps both gates share, in order.
///
/// Both `gate_slice` and `cmd_gate` iterate this one table, so no step can be wired into a single
/// driver and drift green on the path that never runs it. A verification that exists but appears
/// in no row is invoked by nothing and proves nothing; `verify ci-schema-parity` is the tripwire
/// that reds when a row or its dispatch disappears.
const VERIFY_STEPS: &[(&str, &str)] = &[
    ("object registry aliases", "object-registry-aliases"),
    ("wiki seeds", "wiki-seeds"),
    ("faction library seeds", "faction-library-seeds"),
    ("staging compose paths", "staging-compose-paths"),
    ("mission REST size limits", "mission-rest-size-limits"),
    ("CI schema parity", "ci-schema-parity"),
    ("destroy target diagnostics", "destroy-target-diagnostics"),
    ("route tags", "route-tags"),
    ("reporter identity", "results-reporter-identity-comments"),
    ("player identity comments", "player-identity-comments"),
];

/// The repository folder whose workspace members are the tool crates a wave can touch.
const TOOL_MEMBER_FOLDER: &str = "tools/";

/// The tool packages the wave gate's tool lint names whatever else the workspace holds: the
/// `cargo xtask` command router and the developer tool suite.
const ANCHOR_TOOL_PACKAGES: [&str; 2] = ["xtask", "developer_tools"];

/// The packages the wave gate's `clippy xtask+developer_tools` step lints, derived from the
/// workspace under `repo_root`: every member under [`TOOL_MEMBER_FOLDER`], in member-path order.
///
/// A workspace that cannot be read, or that lacks one of [`ANCHOR_TOOL_PACKAGES`] under
/// [`TOOL_MEMBER_FOLDER`], is an error, never a smaller lint.
fn tool_clippy_packages(repo_root: &std::path::Path) -> Result<Vec<String>, String> {
    let members =
        repository_laws::workspace_members::read_workspace_members(repo_root).map_err(|why| {
            format!(
                "the workspace members cannot be read from {} ({why:?})",
                repo_root.join("Cargo.toml").display()
            )
        })?;
    let packages: Vec<String> = members
        .into_iter()
        .filter(|member| member.path.starts_with(TOOL_MEMBER_FOLDER))
        .map(|member| member.package_name)
        .collect();
    if let Some(missing) = ANCHOR_TOOL_PACKAGES
        .iter()
        .find(|anchor| !packages.iter().any(|package| package == **anchor))
    {
        return Err(format!(
            "`{missing}` is no workspace member under `{TOOL_MEMBER_FOLDER}`"
        ));
    }
    Ok(packages)
}

/// The `cargo clippy` command line of the wave gate's host-target lints (the tool crates, the
/// applications and the library crates): one `-p` per package, every target, `-D warnings`.
fn native_clippy_argv(packages: &[String]) -> Vec<&str> {
    let mut argv = vec!["cargo", "clippy"];
    for package in packages {
        argv.extend(["-p", package.as_str()]);
    }
    argv.extend(["--all-targets", "--quiet", "--", "-D", "warnings"]);
    argv
}

#[cfg(test)]
#[path = "tests/gate/tests.rs"]
mod tests;

mod checkrun;
mod clippy_package_sets;
use checkrun::checkrun;
pub(crate) use checkrun::gate_slice;
use checkrun::hostrun;

mod gate_dispatch;
pub(crate) use gate_dispatch::cmd_gate;
