//! The test lane of every workspace member that no dedicated task tests.
//!
//! **Role:** derives, from the root `Cargo.toml` workspace, the packages no dedicated test task
//! covers and runs `cargo test -p <package>` for each, so a member the workspace gains is tested
//! from the moment the root manifest names it. The frontend crates (`crates/frontend/*/*`) are
//! not in this lane: `ci-local-leptos` tests them with `frontend`, in one line
//! ([`crate::frontend_package_lane`]). The API crates (`crates/api/*`) are not in it either: `api-test` and `cargo xtask db test-it` test them with `api`, in the database lane. Their
//! unit tests need no database (none reads `TEST_DATABASE_URL`), so the lane they share with `api`
//! is the one that owns the API family, not a database requirement.
//!
//! **Position:** the `workspace-member-tests` row of [`super::task_runner::TASKS`], which
//! `ci-local` and the ci.yml `workspace-members` job run; the wave gate's `test workspace members`
//! step derives its own package list through [`member_packages_outside_api_family`] with the
//! members its other steps test.
//!
//! **Signals & state:** none; reads the checkout and spawns `cargo test`, one package at a time.
//!
//! **Invariants:** an unreadable workspace and a dedicated entry that names no member are errors,
//! never a smaller lane; each package runs in its own `cargo test`, so no feature unifies across
//! packages (a multi-package run can switch on a feature one package's own build never has, and
//! fail or pass for that reason alone); every package runs even after a red one, and the lane's
//! exit is the first red package's code.

use std::io::Write;
use std::path::Path;

use crate::error::{Error, Result};
use crate::frontend_package_lane::frontend_packages;
use database_operations::local_database::api_test_packages::api_test_packages;
use process_runner::Run;
use repository_laws::workspace_members::read_workspace_members;

use repository_root::find_repository_root;
use verification_core::NotRun;

/// The members a dedicated task of [`super::task_runner::TASKS`] tests, with that task, as
/// `(package, task)`: the API against its database, the frontend (with every crate of its family,
/// which the lane derives) and the offline service worker the `wasm-ci` lane tests with every
/// feature on.
pub const DEDICATED_TEST_TASKS: [(&str, &str); 3] = [
    ("api", "api-test"),
    ("frontend", "ci-local-leptos"),
    ("offline_service_worker", "wasm-ci"),
];

/// The package of every workspace member under `repo_root` except the `dedicated` packages, in
/// member-path order.
///
/// # Errors
/// The workspace cannot be read, or a `dedicated` package is no workspace member (a stale entry
/// would otherwise keep a renamed member out of every lane).
pub fn member_packages_except(repo_root: &Path, dedicated: &[&str]) -> Result<Vec<String>> {
    let members = read_workspace_members(repo_root).map_err(|why| {
        Error::msg(format!(
            "the workspace members cannot be read from {} ({why:?})",
            repo_root.join("Cargo.toml").display()
        ))
    })?;
    if let Some(stale) = dedicated
        .iter()
        .find(|package| !members.iter().any(|m| m.package_name == **package))
    {
        return Err(Error::msg(format!(
            "`{stale}` has a dedicated test step but is no workspace member"
        )));
    }
    Ok(members
        .into_iter()
        .filter(|member| !dedicated.contains(&member.package_name.as_str()))
        .map(|member| member.package_name)
        .collect())
}

/// [`member_packages_except`] without the API family as well: `api` and every member under
/// `crates/api`, which a lane's API step tests together (the derivation
/// `cargo xtask db test-it` runs over).
///
/// # Errors
/// As [`member_packages_except`], or the API packages cannot be derived.
pub fn member_packages_outside_api_family(
    repo_root: &Path,
    dedicated: &[&str],
) -> Result<Vec<String>> {
    let api_family = api_test_packages(repo_root)
        .map_err(|error| Error::msg(format!("the API packages cannot be derived: {error}")))?;
    let mut excluded = dedicated.to_vec();
    excluded.extend(api_family.iter().map(String::as_str));
    member_packages_except(repo_root, &excluded)
}

/// The packages this lane tests under `repo_root`: every member outside [`DEDICATED_TEST_TASKS`],
/// outside the API family that `api-test` tests with `api` and outside the frontend family that
/// `ci-local-leptos` tests with `frontend`, in member-path order.
///
/// # Errors
/// As [`member_packages_outside_api_family`], or the frontend family cannot be derived.
pub fn workspace_member_lane_packages(repo_root: &Path) -> Result<Vec<String>> {
    let frontend_family = frontend_packages(repo_root)?;
    let dedicated: Vec<&str> = DEDICATED_TEST_TASKS
        .iter()
        .map(|(package, _)| *package)
        .chain(frontend_family.iter().map(String::as_str))
        .collect();
    member_packages_outside_api_family(repo_root, &dedicated)
}

/// `cargo xtask ci workspace-member-tests`: `cargo test -p <package>` for every package of
/// [`workspace_member_lane_packages`], each its own run from the repository root.
///
/// Returns 0 when every package passed; otherwise the first red package's exit code (1 for a run
/// killed by a signal, a lane that could not derive its packages, or a cargo that could not start).
pub(crate) fn run() -> i32 {
    match run_every_package() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("workspace-member-tests: {}", crate::cause_chain(&error));
            1
        }
    }
}

fn run_every_package() -> Result<i32> {
    let root = find_repository_root()?;
    let packages = workspace_member_lane_packages(&root)?;
    let mut first_red = 0;
    let mut red = Vec::new();
    for package in &packages {
        println!("cargo test -p {package}");
        // Flush before the child inherits stdout, or the line lands after the output it labels.
        let _ = std::io::stdout().flush();
        // On the inherited terminal: the test run's output streams as it happens.
        let outcome = match Run::new("cargo")
            .args(["test", "-p", package])
            .cwd(&root)
            .terminal()
        {
            Ok(code) => Some(code),
            // A signal is a red run with no exit code, as `ExitStatus::code` reads it.
            Err(NotRun::Signalled { .. }) => None,
            Err(source) => {
                return Err(Error::CargoTestStart {
                    package: package.clone(),
                    source,
                });
            }
        };
        if outcome != Some(0) {
            red.push(package.as_str());
            if first_red == 0 {
                first_red = outcome.filter(|code| *code != 0).unwrap_or(1);
            }
        }
    }
    if red.is_empty() {
        println!(
            "workspace-member-tests: {} package(s) passed: {}",
            packages.len(),
            packages.join(" ")
        );
    } else {
        eprintln!(
            "workspace-member-tests: {} of {} package(s) red: {}",
            red.len(),
            packages.len(),
            red.join(" ")
        );
    }
    Ok(first_red)
}

#[cfg(test)]
#[path = "tests/workspace_member_tests.rs"]
mod tests;
