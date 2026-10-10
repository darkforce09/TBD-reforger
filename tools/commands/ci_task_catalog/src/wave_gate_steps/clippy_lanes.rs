//! The wave gate's clippy lanes and workspace test steps, their package sets derived from the
//! workspace.
//!
//! **Role:** splits the workspace members between the wave gate's four clippy lanes: the tool
//! crates (`clippy xtask+developer_tools`, [`tool_clippy_packages`]), the frontend family
//! (`clippy frontend`, wasm32 and native), the wasm32 members ([`wasm32_clippy_packages`]) and
//! every other application and library crate ([`native_clippy_packages`]).
//! **Position:** the ticket manager's wave gate runs one `mk gate-step` per set; reads the root `Cargo.toml`
//! through [`repository_laws::workspace_members`] and the lane derivations of
//! [`ci_task_catalog`].
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** every workspace member falls in at least one lane, so a member the workspace
//! gains (an application, an API crate, any `crates/**` library) is linted from the moment the
//! root manifest names it; an unreadable workspace, an underivable lane, or a workspace without
//! [`ANCHOR_NATIVE_PACKAGE`] is an error, never a smaller lint.

use std::path::Path;

use crate::frontend_package_lane::frontend_packages_among;
use crate::wasm32_lint_lane::wasm_ci_lint_packages;
use repository_laws::workspace_members::{WorkspaceMember, read_workspace_members};

use super::host;
use super::step_context::Ctx;
use super::{wprint, wprintln};
use crate::frontend_package_lane::frontend_family_argv;
use crate::wasm32_lint_lane::wasm32_clippy_argv;

/// The repository folder whose workspace members are the tool crates a wave can touch.
const TOOL_MEMBER_FOLDER: &str = "tools/";

/// The tool packages the tool lint names whatever else the workspace holds.
const ANCHOR_TOOL_PACKAGES: [&str; 2] = ["xtask", "developer_tools"];

/// The package the native lint names whatever else the workspace holds: the API server.
const ANCHOR_NATIVE_PACKAGE: &str = "api_server";

/// The packages the wave gate's `clippy native crates` step lints for the host target: every
/// workspace member under `repo_root` outside [`TOOL_MEMBER_FOLDER`], outside the frontend family
/// and outside [`wasm32_clippy_packages`], in member-path order. That is the API server with
/// every API crate, the game server host agent, and every other `crates/**` library.
///
/// # Errors
/// The workspace cannot be read, the frontend family or the wasm32 members cannot be derived, or
/// the result lacks [`ANCHOR_NATIVE_PACKAGE`].
fn native_clippy_packages(repo_root: &Path) -> Result<Vec<String>, String> {
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
fn wasm32_clippy_packages(repo_root: &Path) -> Result<Vec<String>, String> {
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

/// The packages of the tool lint: every member under [`TOOL_MEMBER_FOLDER`]; a workspace lacking
/// one of [`ANCHOR_TOOL_PACKAGES`] there is an error, never a smaller lint.
fn tool_clippy_packages(repo_root: &Path) -> Result<Vec<String>, String> {
    let packages: Vec<String> = workspace_members(repo_root)?
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

/// `cargo clippy -p <package>… --all-targets --quiet -- -D warnings`.
fn native_clippy_argv(packages: &[String]) -> Vec<String> {
    let mut argv = host::v(&["cargo", "clippy"]);
    for package in packages {
        argv.push("-p".into());
        argv.push(package.clone());
    }
    argv.extend(host::v(&[
        "--all-targets",
        "--quiet",
        "--",
        "-D",
        "warnings",
    ]));
    argv
}

/// Run a check-class command into the gate's private analysis folder; output printed.
fn checkrun(ctx: &Ctx, argv: &[String]) -> i32 {
    let (out, rc) = host::capture(&ctx.host.checkrun_argv(&ctx.gate_check_target, argv));
    wprint!("{out}");
    rc
}

/// Run a host command; output printed.
fn hostrun(ctx: &Ctx, argv: &[String]) -> i32 {
    let (out, rc) = host::capture(&ctx.host.hostrun_argv(argv));
    wprint!("{out}");
    rc
}

/// A derived package list, or the step's red refusal.
fn derived(packages: Result<Vec<String>, String>, f: impl FnOnce(Vec<String>) -> i32) -> i32 {
    match packages {
        Ok(p) => f(p),
        Err(error) => {
            wprintln!("    {error}");
            1
        }
    }
}

/// `clippy-native`: every application and library crate outside the tools, the frontend family
/// and the wasm32 members, for the host target.
pub(crate) fn clippy_native(ctx: &Ctx) -> i32 {
    derived(native_clippy_packages(&ctx.root), |p| {
        checkrun(ctx, &native_clippy_argv(&p))
    })
}

/// `clippy-wasm32`: the wasm32 members outside the frontend family.
pub(crate) fn clippy_wasm32(ctx: &Ctx) -> i32 {
    derived(wasm32_clippy_packages(&ctx.root), |p| {
        checkrun(ctx, &wasm32_clippy_argv(&p))
    })
}

/// `clippy-tools`: every tool crate.
pub(crate) fn clippy_tools(ctx: &Ctx) -> i32 {
    derived(tool_clippy_packages(&ctx.root), |p| {
        checkrun(ctx, &native_clippy_argv(&p))
    })
}

/// `clippy-frontend` (wasm32) and `clippy-frontend-native`: the frontend family, both halves of
/// its `cfg(target_arch)`, every target.
pub(crate) fn clippy_frontend(ctx: &Ctx, wasm32: bool) -> i32 {
    let trailing: &[&str] = if wasm32 {
        &[
            "--target",
            "wasm32-unknown-unknown",
            "--all-targets",
            "--quiet",
            "--",
            "-D",
            "warnings",
        ]
    } else {
        &[
            "--all-targets",
            "--locked",
            "--quiet",
            "--",
            "-D",
            "warnings",
        ]
    };
    derived(
        frontend_family_argv(&ctx.root, &["cargo", "clippy"], trailing).map_err(|e| e.to_string()),
        |argv| checkrun(ctx, &argv),
    )
}

/// `test-frontend`: the frontend family's tests in the private `gate-frontend` folder.
pub(crate) fn test_frontend(ctx: &Ctx) -> i32 {
    let folder = super::step_context::gate_folder(
        &ctx.main_root,
        repository_layout::build_output::GATE_FRONTEND_SUBFOLDER,
    );
    let assignment = format!("CARGO_TARGET_DIR={folder}");
    derived(
        frontend_family_argv(
            &ctx.root,
            &["env", &assignment, "cargo", "test"],
            &["--quiet"],
        )
        .map_err(|e| e.to_string()),
        |argv| hostrun(ctx, &argv),
    )
}

/// `test-workspace-members`: one `cargo test --workspace` over every member the API and frontend
/// steps do not test, in the private `gate-tools` folder.
pub(crate) fn test_workspace_members(ctx: &Ctx) -> i32 {
    let folder = super::step_context::gate_folder(
        &ctx.main_root,
        repository_layout::build_output::GATE_TOOLS_SUBFOLDER,
    );
    let argv = match crate::workspace_member_tests::workspace_test_argv(&ctx.root) {
        Ok(argv) => argv,
        Err(error) => {
            wprintln!("    {error:#}");
            return 1;
        }
    };
    let mut words = host::v(&[
        "env",
        &format!("CARGO_TARGET_DIR={folder}"),
        "CARGO_INCREMENTAL=0",
    ]);
    words.extend(argv);
    words.push("--quiet".into());
    hostrun(ctx, &words)
}
