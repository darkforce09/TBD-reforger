//! The packages the wasm32 lint covers, derived from the workspace.
//!
//! **Role:** names every workspace package that `cargo clippy --target wasm32-unknown-unknown`
//! lints: each member whose `[package.metadata.layout]` declares `targets = "wasm32"` and every
//! crate of the frontend family (`targets = "any"` ones included, since the browser build compiles
//! their wasm32 half; the single-page app and the offline service worker among them); and splits
//! that set between the two lanes that run it.
//! **Position:** the `wasm-ci` recipe of [`crate::build_lane`] and the `wasm-ci` row of
//! [`crate::task_runner::TASKS`] lint [`wasm_ci_lint_packages`]; the frontend family's own lint
//! runs in `ci-local-leptos` with every target ([`crate::frontend_package_lane`]). Reads
//! [`repository_laws::workspace_members`].
//! **Signals & state:** none; reads the checkout, and the `wasm-ci` row's step spawns one `cargo
//! clippy` through the task runner, with the environment every task line gets.
//! **Invariants:** a wasm32 crate or a frontend crate the workspace gains is linted from the moment
//! its manifest declares the target or the workspace names it, never only once someone extends a
//! list; an unreadable workspace, or a workspace without the single-page app, is an error, never a
//! smaller lint; the two lanes partition the set, each package linted once.

use std::path::Path;

use repository_laws::workspace_members::{WorkspaceMember, read_workspace_members};
use repository_root::find_repository_root;

use crate::error::{Error, Result};
use crate::frontend_package_lane::{frontend_packages_among, is_frontend_member};

/// The `targets` value of `[package.metadata.layout]` that marks a wasm-only crate.
pub const WASM32_LAYOUT_TARGETS: &str = "wasm32";

/// The cargo target the lint builds for.
pub const WASM32_TARGET: &str = "wasm32-unknown-unknown";

/// Every package the wasm32 lint covers under `repo_root`, in member-path order: the declared
/// wasm32 members and the frontend family.
///
/// # Errors
/// The workspace members cannot be read, or the frontend family cannot be derived (a workspace
/// without the single-page app would otherwise leave the browser build unlinted).
pub fn wasm32_lint_packages(repo_root: &Path) -> Result<Vec<String>> {
    let members = read_workspace_members(repo_root).map_err(|why| {
        Error::msg(format!(
            "the workspace members cannot be read from {} ({why:?})",
            repo_root.join("Cargo.toml").display()
        ))
    })?;
    // The family's derivation refuses a workspace without the single-page app.
    frontend_packages_among(&members)?;
    Ok(members
        .iter()
        .filter(|member| is_linted_for_wasm32(member))
        .map(|member| member.package_name.clone())
        .collect())
}

/// The packages the `wasm-ci` lane lints for wasm32: [`wasm32_lint_packages`] without the
/// frontend family, which [`crate::frontend_package_lane::FRONTEND_LANE`] lints with every
/// target.
///
/// # Errors
/// As [`wasm32_lint_packages`], or the frontend family cannot be derived.
pub fn wasm_ci_lint_packages(repo_root: &Path) -> Result<Vec<String>> {
    let members = read_workspace_members(repo_root).map_err(|why| {
        Error::msg(format!(
            "the workspace members cannot be read from {} ({why:?})",
            repo_root.join("Cargo.toml").display()
        ))
    })?;
    let frontend_family = frontend_packages_among(&members)?;
    Ok(wasm32_lint_packages(repo_root)?
        .into_iter()
        .filter(|package| !frontend_family.contains(package))
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

/// True when `member` builds for wasm32: its layout declares the target, or it belongs to the
/// frontend family.
fn is_linted_for_wasm32(member: &WorkspaceMember) -> bool {
    let declared = member
        .manifest
        .layout
        .as_ref()
        .and_then(|layout| layout.targets.as_deref());
    declared == Some(WASM32_LAYOUT_TARGETS) || is_frontend_member(member)
}
