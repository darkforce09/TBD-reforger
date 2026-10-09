//! The API's test, lint and build command lines, derived from the workspace.
//!
//! **Role:** renders the four cargo lines that gate the API, each naming `api_server` and every API
//! crate with one `-p` apiece, and runs them as task-table steps.
//! **Position:** the `api-test`, `rust-test`, `rust-clippy` and `rust-build` rows of
//! [`crate::task_runner::TASKS`] run `run_api_test`, `run_api_unit_tests`, `run_api_clippy` and
//! `run_api_build`; the `mk` lane's `rust-test`, `rust-clippy` and
//! `rust-build` recipes ([`crate::build_lane`]) run the same argv through [`api_line_argv`]. The
//! packages come from `database_operations::local_database::api_test_packages`, the derivation
//! `cargo xtask db test-it` runs over.
//! **Signals & state:** none; reads the checkout, and each runner spawns one cargo through the task
//! runner, with the environment every task line gets.
//! **Invariants:** an API crate is tested, linted and built from the moment the workspace names
//! it, never only once someone extends a list; an unreadable workspace is a red step, never a
//! line naming `api_server` alone; the API family runs in one cargo per line.

use std::path::Path;

use database_operations::local_database::api_test_packages::{
    api_test_packages, package_arguments,
};
use repository_root::find_repository_root;

use crate::error::{Error, Result};

/// One of the API's cargo lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiLine {
    /// `cargo test`: unit and integration tests, honouring `TEST_DATABASE_URL` (`api-test`).
    Test,
    /// `cargo test --lib --bins`: the tests that need no database (`rust-test`).
    UnitTests,
    /// `cargo clippy --all-targets -- -D warnings` (`rust-clippy`).
    Clippy,
    /// `cargo build --all-targets` (`rust-build`).
    Build,
}

impl ApiLine {
    /// The cargo subcommand.
    fn verb(self) -> &'static str {
        match self {
            ApiLine::Test | ApiLine::UnitTests => "test",
            ApiLine::Clippy => "clippy",
            ApiLine::Build => "build",
        }
    }

    /// The words after the `-p` list.
    fn trailing(self) -> &'static [&'static str] {
        match self {
            ApiLine::Test => &[],
            ApiLine::UnitTests => &["--lib", "--bins"],
            ApiLine::Clippy => &["--all-targets", "--", "-D", "warnings"],
            ApiLine::Build => &["--all-targets"],
        }
    }
}

/// `cargo <verb> -p <package>… <trailing words>` for `line` over `packages`.
pub fn api_cargo_argv(line: ApiLine, packages: &[String]) -> Vec<String> {
    let mut argv = vec!["cargo".to_string(), line.verb().to_string()];
    argv.extend(package_arguments(packages));
    argv.extend(line.trailing().iter().map(|word| (*word).to_string()));
    argv
}

/// The argv of `line` over the API packages of the workspace under `repo_root`.
///
/// # Errors
/// The API packages cannot be derived from the workspace.
pub fn api_line_argv(repo_root: &Path, line: ApiLine) -> Result<Vec<String>> {
    let packages = api_test_packages(repo_root)
        .map_err(|error| Error::msg(format!("the API packages cannot be derived: {error}")))?;
    Ok(api_cargo_argv(line, &packages))
}

/// Derives `line` for this checkout, then echoes and runs it as the runner runs any task line.
/// Returns the run's exit code; 1 when the packages could not be derived.
fn run_api_line(line: ApiLine) -> i32 {
    let argv = match find_repository_root()
        .map_err(Error::from)
        .and_then(|root| api_line_argv(&root, line))
    {
        Ok(argv) => argv,
        Err(error) => {
            eprintln!("api lane: {}", crate::cause_chain(&error));
            return 1;
        }
    };
    let words: Vec<&str> = argv.iter().map(String::as_str).collect();
    crate::task_runner::run_derived_line(&words)
}

/// The `api-test` row: [`ApiLine::Test`].
pub(crate) fn run_api_test() -> i32 {
    run_api_line(ApiLine::Test)
}

/// The `rust-test` row: [`ApiLine::UnitTests`].
pub(crate) fn run_api_unit_tests() -> i32 {
    run_api_line(ApiLine::UnitTests)
}

/// The `rust-clippy` row: [`ApiLine::Clippy`].
pub(crate) fn run_api_clippy() -> i32 {
    run_api_line(ApiLine::Clippy)
}

/// The `rust-build` row: [`ApiLine::Build`].
pub(crate) fn run_api_build() -> i32 {
    run_api_line(ApiLine::Build)
}

/// The [`ApiLine`] a task-table step runs, when `run` is one of this module's runners: how the
/// task-table tests read the line a native API step derives.
#[cfg(test)]
pub(crate) fn api_line_of(run: fn() -> i32) -> Option<ApiLine> {
    [
        (run_api_test as fn() -> i32, ApiLine::Test),
        (run_api_unit_tests, ApiLine::UnitTests),
        (run_api_clippy, ApiLine::Clippy),
        (run_api_build, ApiLine::Build),
    ]
    .into_iter()
    .find(|(runner, _)| std::ptr::fn_addr_eq(*runner, run))
    .map(|(_, line)| line)
}

#[cfg(test)]
#[path = "tests/api_package_lane.rs"]
mod tests;
