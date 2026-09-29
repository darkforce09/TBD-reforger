//! The workstation's side of a load run: the keying refreshes and the member load itself.
//!
//! **Role:** the [`WorkstationLoad`] seam and its live implementation, [`LiveWorkstation`]: one
//! invalid refresh sent from a chosen source address with `curl`, and the member load run by the
//! developer-tools load engine.
//!
//! **Position:** held by `LoadProcedure` and the local rehearsal; the tests replace it with a
//! stub that answers at once.
//!
//! **Signals & state:** none; the engine builds and drops its own runtime per run.
//!
//! **Invariants:** the keying refresh carries a fixed invalid token, bypasses every proxy and
//! leaves from the named address; the member load is the engine's [`run`] on the plan as given.

use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, ensure};
use developer_tools::staging_verification::load_generation::{LoadReport, LoadRunPlan, run};
use verification_core::proc::Run;

/// The body of a keying refresh: a refresh token no session was ever issued.
pub(crate) const KEYING_REFRESH_BODY: &str = r#"{"refresh_token":"staging-keying-probe"}"#;
/// The refresh route every keying refresh posts to.
pub(crate) const REFRESH_ROUTE: &str = "/api/v1/auth/refresh";
/// How long one keying refresh may take.
const KEYING_TIMEOUT: Duration = Duration::from_secs(15);

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
            .map_err(|not_run| anyhow!("curl did not run for the keying refresh: {not_run:?}"))?;
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
        run(plan)
    }
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
