//! The test lane of every workspace member that no dedicated task tests.
//!
//! **Role:** derives, from the root `Cargo.toml` workspace, the packages other rows test and runs
//! one `cargo test --workspace` that excludes them, so a member the workspace gains is tested from
//! the moment the root manifest names it. The frontend crates (`crates/frontend/*/*`) are excluded:
//! `ci-local-leptos` tests them, the single-page app `frontend_application` and the offline service
//! worker among them, in one line ([`crate::frontend_package_lane`]). The API crates
//! (`crates/api/*`) are excluded too: `api-test` and `cargo xtask db test-it` test them with
//! `api_server`, in the database lane.
//!
//! **Position:** the `workspace-member-tests` row of [`super::task_runner::TASKS`], which
//! `ci-local` and the ci.yml `workspace-members` job run; the wave gate's `test workspace members`
//! step runs the same command line through [`workspace_test_argv`].
//!
//! **Signals & state:** none; reads the checkout and spawns one `cargo test`.
//!
//! **Invariants:** an unreadable workspace and an excluded package that names no member are
//! errors, never a smaller lane; the lane is one cargo invocation, and its exit is cargo's.

use std::path::Path;

use crate::error::{Error, Result};
use crate::frontend_package_lane::frontend_packages;
use database_operations::local_database::api_test_packages::api_test_packages;
use repository_laws::workspace_members::read_workspace_members;
use repository_root::find_repository_root;

/// The packages other rows test, which this lane excludes: `api_server` with every API crate (the
/// derivation `cargo xtask db test-it` runs over) and the frontend family `ci-local-leptos` tests,
/// derived from the workspace under `repo_root`.
///
/// # Errors
/// The workspace cannot be read, the API or frontend family cannot be derived, or an excluded
/// package is no workspace member (a stale entry would otherwise make cargo refuse the run).
pub fn excluded_packages(repo_root: &Path) -> Result<Vec<String>> {
    let members = read_workspace_members(repo_root).map_err(|why| {
        Error::msg(format!(
            "the workspace members cannot be read from {} ({why:?})",
            repo_root.join("Cargo.toml").display()
        ))
    })?;
    let mut excluded = api_test_packages(repo_root)
        .map_err(|error| Error::msg(format!("the API packages cannot be derived: {error}")))?;
    excluded.extend(frontend_packages(repo_root)?);
    if let Some(stale) = excluded
        .iter()
        .find(|package| !members.iter().any(|m| &m.package_name == *package))
    {
        return Err(Error::msg(format!(
            "`{stale}` is excluded from the workspace test run but is no workspace member"
        )));
    }
    excluded.sort();
    excluded.dedup();
    Ok(excluded)
}

/// `cargo test --workspace --exclude <package>…` over the workspace under `repo_root`, excluding
/// every package of [`excluded_packages`].
///
/// # Errors
/// As [`excluded_packages`].
pub fn workspace_test_argv(repo_root: &Path) -> Result<Vec<String>> {
    let mut argv: Vec<String> = ["cargo", "test", "--workspace"]
        .iter()
        .map(|word| (*word).to_string())
        .collect();
    for package in excluded_packages(repo_root)? {
        argv.push("--exclude".to_string());
        argv.push(package);
    }
    Ok(argv)
}

/// `cargo xtask ci workspace-member-tests`: [`workspace_test_argv`], run once from the repository
/// root. Returns cargo's exit code, or 1 when the lane could not derive its command line.
pub(crate) fn run() -> i32 {
    let argv = match find_repository_root()
        .map_err(Error::from)
        .and_then(|root| workspace_test_argv(&root))
    {
        Ok(argv) => argv,
        Err(error) => {
            eprintln!("workspace-member-tests: {}", crate::cause_chain(&error));
            return 1;
        }
    };
    let words: Vec<&str> = argv.iter().map(String::as_str).collect();
    crate::task_runner::run_derived_line(&words)
}
