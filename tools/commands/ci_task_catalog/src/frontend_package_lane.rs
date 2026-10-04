//! The frontend's format, lint and test command lines, derived from the workspace.
//!
//! **Role:** names the frontend family, the single-page app `frontend` and every workspace member
//! under `crates/frontend/`, and renders the four cargo lines that gate it (format, wasm32 clippy,
//! native clippy, native tests), each naming every package of the family with one `-p` apiece;
//! runs them as task-table steps.
//! **Position:** the `ci-local-leptos` row of [`crate::task_runner::TASKS`] runs
//! `run_frontend_format`, `run_frontend_wasm32_clippy`, `run_frontend_native_clippy` and
//! `run_frontend_tests`; the `mk ci-local-leptos` recipe ([`crate::build_lane`]) runs the same
//! argv through [`frontend_line_argv`]; [`crate::wasm32_lint_lane`] and
//! [`crate::workspace_member_tests`] leave the family to this lane; the wave gate
//! (`platform_execution`) renders its frontend steps through [`frontend_family_argv`]. Reads
//! [`repository_laws::workspace_members`].
//! **Signals & state:** none; reads the checkout, and each runner spawns one cargo through the task
//! runner, with the environment every task line gets.
//! **Invariants:** a frontend crate is formatted, linted for both targets and tested from the
//! moment the workspace names it, never only once someone extends a list; the app comes first,
//! then the crates in member-path order; an unreadable workspace, or a workspace without the app,
//! is an error, never a line naming fewer packages; the family runs in one cargo per line, as the
//! API family does.

use std::path::Path;

use repository_laws::workspace_members::{WorkspaceMember, read_workspace_members};
use repository_layout::find_repository_root;

use crate::error::{Error, Result};

/// The package of the single-page app, the root of the frontend family.
pub const FRONTEND_APPLICATION: &str = "frontend";

/// The folder whose workspace members are the frontend's library crates, one per
/// `crates/frontend/<layer>/<crate>`.
pub const FRONTEND_CRATES_FOLDER: &str = "crates/frontend";

/// The task-table row and `mk` recipe that format, lint and test the frontend family.
pub const FRONTEND_LANE: &str = "ci-local-leptos";

/// One of the frontend family's cargo lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendLine {
    /// `cargo fmt … --check`.
    Format,
    /// `cargo clippy … --target wasm32-unknown-unknown --all-targets -- -D warnings`: the browser
    /// build's half of every `cfg(target_arch)` split.
    Wasm32Clippy,
    /// `cargo clippy … --all-targets --locked -- -D warnings`: the native test build's half.
    NativeClippy,
    /// `cargo test …`: the native test suites.
    Test,
}

impl FrontendLine {
    /// The words before the `-p` list.
    fn leading(self) -> &'static [&'static str] {
        match self {
            FrontendLine::Format => &["cargo", "fmt"],
            FrontendLine::Wasm32Clippy | FrontendLine::NativeClippy => &["cargo", "clippy"],
            FrontendLine::Test => &["cargo", "test"],
        }
    }

    /// The words after the `-p` list.
    fn trailing(self) -> &'static [&'static str] {
        match self {
            FrontendLine::Format => &["--check"],
            FrontendLine::Wasm32Clippy => &[
                "--target",
                "wasm32-unknown-unknown",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            FrontendLine::NativeClippy => &["--all-targets", "--locked", "--", "-D", "warnings"],
            FrontendLine::Test => &[],
        }
    }
}

/// True when `member` belongs to the frontend family: the app, or a member under
/// [`FRONTEND_CRATES_FOLDER`].
pub fn is_frontend_member(member: &WorkspaceMember) -> bool {
    member.package_name == FRONTEND_APPLICATION
        || member
            .path
            .strip_prefix(FRONTEND_CRATES_FOLDER)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// The frontend family among `members`: the app first, then every member under
/// [`FRONTEND_CRATES_FOLDER`] in member-path order.
///
/// # Errors
/// The app is not among `members` (a renamed app would otherwise leave every frontend lane).
pub fn frontend_packages_among(members: &[WorkspaceMember]) -> Result<Vec<String>> {
    if !members
        .iter()
        .any(|member| member.package_name == FRONTEND_APPLICATION)
    {
        return Err(Error::msg(format!(
            "`{FRONTEND_APPLICATION}` heads the frontend lanes but is no workspace member"
        )));
    }
    let mut crates: Vec<&WorkspaceMember> = members
        .iter()
        .filter(|member| member.package_name != FRONTEND_APPLICATION && is_frontend_member(member))
        .collect();
    crates.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(std::iter::once(FRONTEND_APPLICATION.to_string())
        .chain(crates.into_iter().map(|member| member.package_name.clone()))
        .collect())
}

/// The frontend family of the workspace under `repo_root`, as [`frontend_packages_among`] orders
/// it.
///
/// # Errors
/// The workspace members cannot be read, or as [`frontend_packages_among`].
pub fn frontend_packages(repo_root: &Path) -> Result<Vec<String>> {
    let members = read_workspace_members(repo_root).map_err(|why| {
        Error::msg(format!(
            "the workspace members cannot be read from {} ({why:?})",
            repo_root.join("Cargo.toml").display()
        ))
    })?;
    frontend_packages_among(&members)
}

/// `leading`, then `-p <package>` for every package, then `trailing`.
pub fn package_argv(leading: &[&str], packages: &[String], trailing: &[&str]) -> Vec<String> {
    let mut argv: Vec<String> = leading.iter().map(|word| (*word).to_string()).collect();
    for package in packages {
        argv.push("-p".to_string());
        argv.push(package.clone());
    }
    argv.extend(trailing.iter().map(|word| (*word).to_string()));
    argv
}

/// `leading`, a `-p` for every package of the frontend family under `repo_root`, then
/// `trailing`: how a caller with its own flags (the wave gate's `--quiet`, a target-directory
/// prefix) names the family.
///
/// # Errors
/// As [`frontend_packages`].
pub fn frontend_family_argv(
    repo_root: &Path,
    leading: &[&str],
    trailing: &[&str],
) -> Result<Vec<String>> {
    Ok(package_argv(
        leading,
        &frontend_packages(repo_root)?,
        trailing,
    ))
}

/// The argv of `line` over `packages`.
pub fn frontend_cargo_argv(line: FrontendLine, packages: &[String]) -> Vec<String> {
    package_argv(line.leading(), packages, line.trailing())
}

/// The argv of `line` over the frontend family of the workspace under `repo_root`.
///
/// # Errors
/// As [`frontend_packages`].
pub fn frontend_line_argv(repo_root: &Path, line: FrontendLine) -> Result<Vec<String>> {
    Ok(frontend_cargo_argv(line, &frontend_packages(repo_root)?))
}

/// Derives `line` for this checkout, then echoes and runs it as the runner runs any task line.
/// Returns the run's exit code; 1 when the packages could not be derived.
fn run_frontend_line(line: FrontendLine) -> i32 {
    let argv = match find_repository_root()
        .map_err(Error::from)
        .and_then(|root| frontend_line_argv(&root, line))
    {
        Ok(argv) => argv,
        Err(error) => {
            eprintln!("frontend lane: {}", crate::cause_chain(&error));
            return 1;
        }
    };
    let words: Vec<&str> = argv.iter().map(String::as_str).collect();
    crate::task_runner::run_derived_line(&words)
}

/// The `ci-local-leptos` row's format step: [`FrontendLine::Format`].
pub(crate) fn run_frontend_format() -> i32 {
    run_frontend_line(FrontendLine::Format)
}

/// The `ci-local-leptos` row's wasm32 lint: [`FrontendLine::Wasm32Clippy`].
pub(crate) fn run_frontend_wasm32_clippy() -> i32 {
    run_frontend_line(FrontendLine::Wasm32Clippy)
}

/// The `ci-local-leptos` row's native lint: [`FrontendLine::NativeClippy`].
pub(crate) fn run_frontend_native_clippy() -> i32 {
    run_frontend_line(FrontendLine::NativeClippy)
}

/// The `ci-local-leptos` row's test step: [`FrontendLine::Test`].
pub(crate) fn run_frontend_tests() -> i32 {
    run_frontend_line(FrontendLine::Test)
}

/// The [`FrontendLine`] a task-table step runs, when `run` is one of this module's runners: how
/// the task-table tests read the line a native frontend step derives.
#[cfg(test)]
pub(crate) fn frontend_line_of(run: fn() -> i32) -> Option<FrontendLine> {
    [
        (run_frontend_format as fn() -> i32, FrontendLine::Format),
        (run_frontend_wasm32_clippy, FrontendLine::Wasm32Clippy),
        (run_frontend_native_clippy, FrontendLine::NativeClippy),
        (run_frontend_tests, FrontendLine::Test),
    ]
    .into_iter()
    .find(|(runner, _)| std::ptr::fn_addr_eq(*runner, run))
    .map(|(_, line)| line)
}

#[cfg(test)]
#[path = "tests/frontend_package_lane.rs"]
mod tests;
