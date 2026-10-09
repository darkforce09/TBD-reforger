//! The package sets the wave gate's clippy steps lint, derived from the workspace.
//!
//! **Role:** splits the workspace members between the wave gate's four clippy lanes: the tool
//! crates (`clippy xtask+developer_tools`, [`super::tool_clippy_packages`]), the frontend family
//! (`clippy frontend`, wasm32 and native), the wasm32 members ([`wasm32_clippy_packages`]) and
//! every other application and library crate ([`native_clippy_packages`]).
//! **Position:** `gate_dispatch::cmd_gate` runs one step per set; reads the root `Cargo.toml`
//! through [`repository_laws::workspace_members`] and the lane derivations of
//! [`ci_task_catalog`].
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** every workspace member falls in at least one lane, so a member the workspace
//! gains (an application, an API crate, any `crates/**` library) is linted from the moment the
//! root manifest names it; an unreadable workspace, an underivable lane, or a workspace without
//! [`ANCHOR_NATIVE_PACKAGE`] is an error, never a smaller lint.

use std::path::Path;

use ci_task_catalog::frontend_package_lane::frontend_packages_among;
use ci_task_catalog::wasm32_lint_lane::wasm_ci_lint_packages;
use repository_laws::workspace_members::{WorkspaceMember, read_workspace_members};

use super::TOOL_MEMBER_FOLDER;

/// The package the native lint names whatever else the workspace holds: the API server.
pub(super) const ANCHOR_NATIVE_PACKAGE: &str = "api_server";

/// The packages the wave gate's `clippy apps and crates` step lints for the host target: every
/// workspace member under `repo_root` outside [`TOOL_MEMBER_FOLDER`], outside the frontend family
/// and outside [`wasm32_clippy_packages`], in member-path order. That is the API server with
/// every API crate, the game server host agent, and every other `crates/**` library.
///
/// # Errors
/// The workspace cannot be read, the frontend family or the wasm32 members cannot be derived, or
/// the result lacks [`ANCHOR_NATIVE_PACKAGE`].
pub(super) fn native_clippy_packages(repo_root: &Path) -> Result<Vec<String>, String> {
    let members = workspace_members(repo_root)?;
    let frontend_family = frontend_packages_among(&members).map_err(|error| error.to_string())?;
    let wasm32 = wasm32_clippy_packages(repo_root)?;
    let packages: Vec<String> = members
        .into_iter()
        .filter(|member| !member.path.starts_with(TOOL_MEMBER_FOLDER))
        .map(|member| member.package_name)
        .filter(|package| !frontend_family.contains(package) && !wasm32.contains(package))
        .collect();
    if !packages
        .iter()
        .any(|package| package == ANCHOR_NATIVE_PACKAGE)
    {
        return Err(format!(
            "`{ANCHOR_NATIVE_PACKAGE}` is no workspace member outside `{TOOL_MEMBER_FOLDER}`, the \
             frontend family and the wasm32 members"
        ));
    }
    Ok(packages)
}

/// The packages the wave gate's `clippy wasm32 members` step lints for
/// `wasm32-unknown-unknown`: the `wasm-ci` lane's set, every member whose layout declares
/// `targets = "wasm32"` outside the frontend family (the offline service worker is in the family).
///
/// # Errors
/// The `wasm-ci` lane cannot derive its packages.
pub(super) fn wasm32_clippy_packages(repo_root: &Path) -> Result<Vec<String>, String> {
    wasm_ci_lint_packages(repo_root).map_err(|error| error.to_string())
}

/// The workspace members under `repo_root`, or the reason they cannot be read.
fn workspace_members(repo_root: &Path) -> Result<Vec<WorkspaceMember>, String> {
    read_workspace_members(repo_root).map_err(|why| {
        format!(
            "the workspace members cannot be read from {} ({why:?})",
            repo_root.join("Cargo.toml").display()
        )
    })
}
