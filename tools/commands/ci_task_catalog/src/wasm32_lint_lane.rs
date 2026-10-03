//! The packages the wasm32 lint covers, derived from the workspace.
//!
//! **Role:** names every workspace package that `cargo clippy --target wasm32-unknown-unknown`
//! lints: each member whose `[package.metadata.layout]` declares `targets = "wasm32"`, the two
//! browser applications that carry no layout table; and splits that set between the two lanes that run it.
//! **Position:** the `wasm-ci` recipe of [`crate::build_lane`] and the `wasm-ci` row of
//! [`crate::task_runner::TASKS`] lint [`wasm_ci_lint_packages`]; the frontend's own lint runs in
//! `ci-local-leptos` with every target. Reads [`repository_laws::workspace_members`].
//! **Signals & state:** none; reads the checkout, and the `wasm-ci` row's step spawns one `cargo
//! clippy` through the task runner, with the environment every task line gets.
//! **Invariants:** a wasm32 crate the workspace gains is linted from the moment its manifest
//! declares the target, never only once someone extends a list; an unreadable workspace, or a
//! named browser application that is no workspace member, is an error, never a smaller lint.

use std::path::Path;

use repository_laws::workspace_members::{WorkspaceMember, read_workspace_members};
use repository_layout::find_repository_root;

use crate::error::{Error, Result};

/// The `targets` value of `[package.metadata.layout]` that marks a wasm-only crate.
pub const WASM32_LAYOUT_TARGETS: &str = "wasm32";

/// The browser applications that ship to wasm32 without a layout table: the single-page app and
/// the offline service worker. Each must be a workspace member.
pub const BROWSER_APPLICATIONS: [&str; 2] = ["frontend", "offline_service_worker"];

/// The packages whose wasm32 lint runs in a lane of their own, with that lane: the frontend in
/// `ci-local-leptos`, which lints every target of it.
pub const OWN_LANE_WASM32_LINTS: [(&str, &str); 1] = [("frontend", "ci-local-leptos")];

/// The cargo target the lint builds for.
pub const WASM32_TARGET: &str = "wasm32-unknown-unknown";

/// Every package the wasm32 lint covers under `repo_root`, in member-path order.
///
/// # Errors
/// The workspace members cannot be read, or one of [`BROWSER_APPLICATIONS`] is no workspace
/// member (a renamed application would otherwise leave the lint).
pub fn wasm32_lint_packages(repo_root: &Path) -> Result<Vec<String>> {
    let members = read_workspace_members(repo_root).map_err(|why| {
        Error::msg(format!(
            "the workspace members cannot be read from {} ({why:?})",
            repo_root.join("Cargo.toml").display()
        ))
    })?;
    if let Some(missing) = BROWSER_APPLICATIONS
        .iter()
        .find(|package| !members.iter().any(|m| m.package_name == **package))
    {
        return Err(Error::msg(format!(
            "`{missing}` is linted for wasm32 but is no workspace member"
        )));
    }
    Ok(members
        .iter()
        .filter(|member| is_linted_for_wasm32(member))
        .map(|member| member.package_name.clone())
        .collect())
}

/// The packages the `wasm-ci` lane lints for wasm32: [`wasm32_lint_packages`] without the
/// packages of [`OWN_LANE_WASM32_LINTS`].
///
/// # Errors
/// As [`wasm32_lint_packages`].
pub fn wasm_ci_lint_packages(repo_root: &Path) -> Result<Vec<String>> {
    Ok(wasm32_lint_packages(repo_root)?
        .into_iter()
        .filter(|package| {
            !OWN_LANE_WASM32_LINTS
                .iter()
                .any(|(own_lane, _)| own_lane == package)
        })
        .collect())
}

/// The `cargo clippy` command line of the `wasm-ci` lane's wasm32 lint: one `-p` per package, the
/// wasm32 target, `-D warnings`.
pub fn wasm32_clippy_argv(packages: &[String]) -> Vec<String> {
    let mut argv = vec!["cargo".to_string(), "clippy".to_string()];
    for package in packages {
        argv.push("-p".to_string());
        argv.push(package.clone());
    }
    for word in ["--target", WASM32_TARGET, "--", "-D", "warnings"] {
        argv.push(word.to_string());
    }
    argv
}

/// The wasm32 lint step of the `wasm-ci` row of [`crate::task_runner::TASKS`]: derives the
/// command line, then echoes and runs it as the runner runs any task line.
///
/// Returns the run's exit code, as a task line's; 1 when the lane could not derive its packages.
pub(crate) fn run_wasm_ci_lint() -> i32 {
    let argv = match find_repository_root()
        .map_err(Error::from)
        .and_then(|root| wasm_ci_lint_packages(&root))
    {
        Ok(packages) => wasm32_clippy_argv(&packages),
        Err(error) => {
            eprintln!("wasm-ci: {}", crate::cause_chain(&error));
            return 1;
        }
    };
    let words: Vec<&str> = argv.iter().map(String::as_str).collect();
    crate::task_runner::run_derived_line(&words)
}

/// True when `member` builds for wasm32: its layout declares the target, or it is one of the
/// browser applications, which carry no layout table.
fn is_linted_for_wasm32(member: &WorkspaceMember) -> bool {
    let declared = member
        .manifest
        .layout
        .as_ref()
        .and_then(|layout| layout.targets.as_deref());
    let package = member.package_name.as_str();
    declared == Some(WASM32_LAYOUT_TARGETS) || BROWSER_APPLICATIONS.contains(&package)
}

#[cfg(test)]
#[path = "tests/wasm32_lint_lane.rs"]
mod tests;
