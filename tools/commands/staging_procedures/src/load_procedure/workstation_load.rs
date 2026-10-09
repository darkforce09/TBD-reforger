//! The workstation's side of a load run: the keying refreshes and the member load itself.
//!
//! **Role:** the [`WorkstationLoad`] seam and its live implementation, [`LiveWorkstation`]: one
//! invalid refresh sent from a chosen source address with `curl`, and the member load run by the
//! `staging-load` executable as a child process.
//!
//! **Position:** held by `LoadProcedure` and the local rehearsal. The member load builds `developer_tools`' `staging-load` binary
//! into the cargo target folder, writes the plan to its standard input and reads the report from
//! its standard output, both through `staging_load_plan`'s process-boundary codec, so the
//! generator's tokio runtime and HTTP client never enter xtask.
//!
//! **Signals & state:** none; the child builds and drops its own runtime per run.
//!
//! **Invariants:** the keying refresh carries a fixed invalid token, bypasses every proxy and
//! leaves from the named address; the member load runs the plan as given, and a child that exits
//! non-zero or prints no report is an error carrying the child's standard error.

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Result, ResultExt, ensure, refusal};
use process_runner::Run;
use repository_layout::BUILD_OUTPUT_FOLDER;
use repository_root::find_repository_root;
use staging_load_plan::{LoadReport, LoadRunPlan, decode_report, encode_plan};

/// The body of a keying refresh: a refresh token no session was ever issued.
pub(crate) const KEYING_REFRESH_BODY: &str = r#"{"refresh_token":"staging-keying-probe"}"#;
/// The refresh route every keying refresh posts to.
pub(crate) const REFRESH_ROUTE: &str = "/api/v1/auth/refresh";
/// How long one keying refresh may take.
const KEYING_TIMEOUT: Duration = Duration::from_secs(15);
/// The package that holds the member load executable.
const STAGING_LOAD_PACKAGE: &str = "developer_tools";
/// The member load executable: a plan as JSON on standard input, the report as JSON on standard
/// output.
const STAGING_LOAD_BINARY: &str = "staging-load";

/// What a load run does on the workstation.
pub(crate) trait WorkstationLoad: Send + Sync {
    /// Posts [`KEYING_REFRESH_BODY`] to `origin`'s refresh route from `address` and returns the
    /// answer's status.
    fn keying_refresh(&self, origin: &str, address: IpAddr) -> Result<u16>;

    /// Runs the member load `plan` describes and returns what it measured.
    fn member_load(&self, plan: &LoadRunPlan) -> Result<LoadReport>;
}

/// The live workstation: `curl` and the load engine.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LiveWorkstation;

impl WorkstationLoad for LiveWorkstation {
    fn keying_refresh(&self, origin: &str, address: IpAddr) -> Result<u16> {
        let output = Run::new("curl")
            .args(keying_arguments(origin, address))
            .timeout(KEYING_TIMEOUT)
            .output()
            .map_err(|not_run| refusal!("curl did not run for the keying refresh: {not_run:?}"))?;
        ensure!(
            output.code == 0,
            "curl exited {} for the keying refresh from {address}: {}",
            output.code,
            output.stderr.trim()
        );
        output
            .stdout
            .trim()
            .parse()
            .with_context(|| format!("curl printed no status for {address}"))
    }

    fn member_load(&self, plan: &LoadRunPlan) -> Result<LoadReport> {
        let binary = build_staging_load(&find_repository_root()?)?;
        let output = Run::new(&binary)
            .stdin(encode_plan(plan)?)
            .output()
            .map_err(|not_run| refusal!("{STAGING_LOAD_BINARY} did not run: {not_run:?}"))?;
        ensure!(
            output.code == 0,
            "{STAGING_LOAD_BINARY} exited {}: {}",
            output.code,
            output.stderr.trim()
        );
        Ok(decode_report(&output.stdout)?)
    }
}

/// Build the `staging-load` executable quietly into the cargo target folder (`CARGO_TARGET_DIR`,
/// else `<root>/target`) and return where it landed.
fn build_staging_load(root: &Path) -> Result<PathBuf> {
    let target_folder = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| root.join(BUILD_OUTPUT_FOLDER), PathBuf::from);
    let built = Run::new("cargo")
        .args(staging_load_build_arguments())
        .cwd(root)
        .output()
        .map_err(|not_run| {
            refusal!("cargo did not run to build {STAGING_LOAD_BINARY}: {not_run:?}")
        })?;
    ensure!(
        built.code == 0,
        "cargo exited {} building {STAGING_LOAD_BINARY}: {}",
        built.code,
        built.stderr.trim()
    );
    let binary = target_folder.join("debug").join(STAGING_LOAD_BINARY);
    ensure!(
        binary.is_file(),
        "cargo built {STAGING_LOAD_BINARY}, but {} is not there",
        binary.display()
    );
    Ok(binary)
}

/// cargo's arguments for building the `staging-load` executable.
pub(crate) fn staging_load_build_arguments() -> [&'static str; 6] {
    [
        "build",
        "-q",
        "-p",
        STAGING_LOAD_PACKAGE,
        "--bin",
        STAGING_LOAD_BINARY,
    ]
}

/// `curl`'s arguments for one keying refresh from `address`.
pub(crate) fn keying_arguments(origin: &str, address: IpAddr) -> Vec<String> {
    vec![
        "--silent".into(),
        "--show-error".into(),
        "--noproxy".into(),
        "*".into(),
        "--max-time".into(),
        "10".into(),
        "--interface".into(),
        address.to_string(),
        "--output".into(),
        "/dev/null".into(),
        "--write-out".into(),
        "%{http_code}".into(),
        "--request".into(),
        "POST".into(),
        "--header".into(),
        "Content-Type: application/json".into(),
        "--data-binary".into(),
        KEYING_REFRESH_BODY.into(),
        format!("{}{REFRESH_ROUTE}", origin.trim_end_matches('/')),
    ]
}
