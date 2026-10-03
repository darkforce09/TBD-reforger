//! ssh / rsync / systemd transport and the deploy pipeline.
//!
//! **Role:** the staging deploy's transport and pipeline: ssh, rsync and systemd argv, the
//! [`Runner`] that executes or prints them, and the per-instance boot verdict.
//! **Position:** a child of [`crate::staging`], whose dispatch calls `deploy`; it uses
//! [`super::boot`], [`super::config`] and [`super::fleet_instances`].
//! **Signals & state:** the [`Runner`], which holds the dry-run switch for one deploy.
//! **Invariants:** a dry run opens no socket; no secret reaches an argument vector or a printed
//! line; the rsync excludes every reference lane.
//!
//! ── NOTHING IN THIS FILE RUNS OUTSIDE A LIVE DEPLOY ──────────────────────────────────────────
//!
//! `deploy/deploy.env` is absent on every development machine — it is gitignored
//! AND rsync-excluded by design, so the credential exists only on the operator's PC. Every
//! function below that spawns `ssh`, `rsync`, `systemctl` or `curl` is therefore
//! live-unverified. What IS verified:
//!
//! * the exact program + argument vector, in order, for every spawn — `tests`;
//! * the exact stdin payload for every `ssh … bash -s` heredoc — `tests`;
//! * the four-outcome exit-code read of `mod remote-logs` — `v6_verdict` + `tests`;
//! * the whole `--dry-run` plan for every instance, which opens no socket (see [`Runner`]);
//! * that no secret reaches an argument vector or a printed line — `tests`.
//!
//! What is NOT verified: whether a real `ssh` accepts these argv, whether the remote `bash -s`
//! payloads behave on the host, whether the website API answers there, and whether the boot wait
//! loop's timing assumptions hold. Only the operator running a live deploy exercises those, so
//! this is a statement about the test environment, not about the code.
//!
//! ── THE EXCLUDE LIST IS A LICENCE BOUNDARY, NOT AN OPTIMISATION ──────────────────────────────
//!
//! EXCLUDE EVERY ORACLE LANE, not just CRF. These are read-only reference trees; the
//! server only ever runs `apps/mod/tbd-framework` (see the addon symlink), so shipping them is
//! pure licence exposure for zero benefit. Every lane lives in `apps/mod/References/`, and both
//! rsync builders exclude that whole folder with one rule, so a lane added there is excluded
//! without an edit here. In the MAIN checkout (which is what deploys) the lanes are real
//! directories, not the worktree symlinks, and they hold ~30 MB of carved Bohemia game source.
//! `playable_selector` has NO LICENCE AT ALL, so copying it to a server is redistribution we have
//! no permission for.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use crate::error::Result;
use process_runner::Run;
use verification_core::verdict::NotRun;

use super::boot::{self, Out};
use super::config::Env;
use super::fleet_instances::FleetInstance;
use super::{Cli, Paths};
use process_runner::secure_shell_transport::{SshBase, ssh_argv};

/// The bash `run()` wrapper: echo under `--dry-run`, execute otherwise.
///
/// LATENT FINDING, reported not fixed: in the shell script this dry-run branch was **dead code**.
/// Every call site of `ssh_cmd` / `rsync_to_remote` already sat inside an explicit
/// `if [ "$DRY_RUN" -eq 1 ]` arm that printed a bespoke `[dry-run] …` line and skipped the call, so
/// `run`'s generic `echo "[dry-run] $*"` could never fire. It is kept — with the same shape — so
/// that a call site added later without its own guard still cannot reach the network. Removing it
/// would convert a latent no-op into a live `ssh`.
pub(super) struct Runner {
    dry_run: bool,
}

impl Runner {
    /// `ssh_cmd` — `run "${SSH_BASE[@]}" "$TBD_SSH_HOST" "$@"`, with optional stdin.
    ///
    /// Returns the raw exit status. `NotRun` (ssh absent, killed by a signal, timed out) is mapped
    /// through [`not_run_exit`] so it can never fold into "the step succeeded".
    fn ssh(
        &self,
        base: &SshBase,
        host: &str,
        remote: &[String],
        stdin: Option<String>,
    ) -> Result<i32, u8> {
        let argv = ssh_argv(base, host, remote);
        if self.dry_run {
            println!("[dry-run] {}", argv.join(" "));
            return Ok(0);
        }
        let (program, _) = base.program_args();
        if let Err(e) = process_runner::which(&program) {
            return Err(not_run_exit(&e));
        }
        let mut run = base.with_password(Run::new(&program));
        for a in argv.iter().skip(1) {
            run = run.arg(a);
        }
        if let Some(body) = stdin {
            run = run.stdin(body);
        }
        // `merged_output` is real `2>&1`: the bash let both streams reach the terminal, and a
        // remote step's diagnosis is usually split across them.
        match run.timeout(Duration::from_secs(3600)).merged_output() {
            Ok(out) => {
                let _ = io::stdout().write_all(out.text.as_bytes());
                let _ = io::stdout().flush();
                Ok(out.code)
            }
            Err(e) => Err(not_run_exit(&e)),
        }
    }

    /// `ssh_cmd` where a non-zero status must abort the deploy, as `set -e` did.
    fn ssh_ok(
        &self,
        base: &SshBase,
        host: &str,
        remote: &[String],
        stdin: Option<String>,
    ) -> Result<(), u8> {
        match self.ssh(base, host, remote, stdin)? {
            0 => Ok(()),
            code => Err(code as u8),
        }
    }

    /// `ssh_cmd "…"` capturing stdout, for the boot-verify probes, with optional stdin.
    fn ssh_capture(
        &self,
        base: &SshBase,
        host: &str,
        remote: &[String],
        stdin: Option<String>,
    ) -> Result<(i32, String), u8> {
        let argv = ssh_argv(base, host, remote);
        if self.dry_run {
            println!("[dry-run] {}", argv.join(" "));
            return Ok((0, String::new()));
        }
        let (program, _) = base.program_args();
        if let Err(e) = process_runner::which(&program) {
            return Err(not_run_exit(&e));
        }
        let mut run = base.with_password(Run::new(&program));
        for a in argv.iter().skip(1) {
            run = run.arg(a);
        }
        if let Some(body) = stdin {
            run = run.stdin(body);
        }
        match run.timeout(Duration::from_secs(600)).output() {
            Ok(out) => Ok((out.code, out.stdout)),
            Err(e) => Err(not_run_exit(&e)),
        }
    }
}

/// `["bash", "-s"]`: the remote command every payload runs under.
fn bash_stdin() -> Vec<String> {
    vec!["bash".to_string(), "-s".to_string()]
}

#[cfg(test)]
#[path = "tests/remote/tests.rs"]
mod tests;

mod ssh_argv;
use ssh_argv::not_run_exit;

mod fleet_deploy;
pub(super) use fleet_deploy::deploy;
#[cfg(test)]
use fleet_deploy::dry_run_plan;

mod instance_boot_verdict;
use instance_boot_verdict::instance_boot_verdicts;

mod deployed_scenario;
use deployed_scenario::deployed_scenario;
#[cfg(test)]
pub(crate) use deployed_scenario::{scenario_of_config, scenario_read_payload};

mod website_api_health_check;
#[cfg(test)]
use website_api_health_check::website_api_refusal;
use website_api_health_check::{require_website_api, website_api_health_url};

#[cfg(test)]
pub(crate) use ssh_argv::{rsync_argv, v6_verdict};
