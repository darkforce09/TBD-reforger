//! The packages the API's test, lint and build lanes cover, derived from the workspace.
//!
//! **Role:** names the API server package (`api_server`, the package that assembles the API
//! crates into the binary and holds the integration test binaries), every workspace member under
//! `crates/api`, and every other member whose build or tests use an API crate (the staging
//! fixtures tool, which writes through the API's services and whose suites need the API's
//! database), and spells them as cargo `-p` arguments.
//! **Position:** a child of [`crate::local_database`]; [`super::test_it`] runs the suite over
//! these packages, and the CI task table, the `mk` build lane and the wave gate's `test api` step
//! derive their API lines through it. Reads [`repository_laws::workspace_members`].
//! **Signals & state:** none; reads the root manifest and every member manifest.
//! **Invariants:** an API crate, and a member that uses one, is covered from the moment the
//! workspace names it, never only once someone extends a list; an unreadable workspace, or one
//! without the `api_server` package, is an error, never a smaller lane; `api_server` comes first,
//! then the other API crates, then the members that use them, each group in member-path order;
//! every package appears once, although `api_server` itself sits under `crates/api`; a
//! build-script edge alone does not make a member a user of the API crates.

use std::path::Path;

use repository_laws::cargo_manifest::DependencyKind;
use repository_laws::workspace_members::{WorkspaceMember, read_workspace_members};

use crate::error::{Error, Result};

/// The API server package: the crate that assembles the API crates into the `api-server` binary
/// and holds the integration test binaries.
pub const API_APPLICATION_PACKAGE: &str = "api_server";

/// The folder whose workspace members are the API crates.
pub const API_CRATES_FOLDER: &str = "crates/api";

/// [`API_APPLICATION_PACKAGE`], then the package of every other workspace member directly under
/// [`API_CRATES_FOLDER`], then the package of every other member with a normal or development
/// dependency on one of those API crates, each group in member-path order and every package once.
///
/// # Errors
/// The workspace members cannot be read, or [`API_APPLICATION_PACKAGE`] is no workspace member.
pub fn api_test_packages(repo_root: &Path) -> Result<Vec<String>> {
    let members = read_workspace_members(repo_root).map_err(|why| {
        Error::msg(format!(
            "the workspace members cannot be read from {} ({why:?})",
            repo_root.join("Cargo.toml").display()
        ))
    })?;
    if !members
        .iter()
        .any(|member| member.package_name == API_APPLICATION_PACKAGE)
    {
        return Err(Error::msg(format!(
            "`{API_APPLICATION_PACKAGE}` is no workspace member of {}",
            repo_root.join("Cargo.toml").display()
        )));
    }
    let (api_crates, other_members): (Vec<WorkspaceMember>, Vec<WorkspaceMember>) = members
        .into_iter()
        .partition(|member| member.parent_folder() == API_CRATES_FOLDER);
    let api_crate_names: Vec<String> = api_crates
        .into_iter()
        .map(|member| member.package_name)
        .collect();
    let api_crate_users = other_members
        .into_iter()
        .filter(|member| uses_an_api_crate(member, &api_crate_names))
        .map(|member| member.package_name);
    let mut packages = vec![API_APPLICATION_PACKAGE.to_string()];
    for package in api_crate_names.iter().cloned().chain(api_crate_users) {
        if !packages.contains(&package) {
            packages.push(package);
        }
    }
    Ok(packages)
}

/// Whether `member` has a normal or development dependency on a package of `api_crate_names`.
fn uses_an_api_crate(member: &WorkspaceMember, api_crate_names: &[String]) -> bool {
    member
        .manifest
        .dependencies
        .iter()
        .any(|edge| edge.kind != DependencyKind::Build && api_crate_names.contains(&edge.package))
}

/// One `-p <package>` pair per package, in order.
pub fn package_arguments(packages: &[String]) -> Vec<String> {
    packages
        .iter()
        .flat_map(|package| ["-p".to_string(), package.clone()])
        .collect()
}

#[cfg(test)]
#[path = "tests/api_test_packages/tests.rs"]
mod tests;
