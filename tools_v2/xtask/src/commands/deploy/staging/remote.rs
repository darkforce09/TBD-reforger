//! ssh / rsync / compose / systemd transport and the deploy pipeline (bash lines 1524–1889).
//!
//! ── NOTHING IN THIS FILE HAS BEEN EXECUTED ───────────────────────────────────────────────────
//!
//! `scripts/deploy/deploy.env` is absent on every machine this port was written on — it is
//! gitignored AND rsync-excluded by design, so the credential exists only on a developer's PC.
//! Every function below that spawns `ssh`, `rsync`, `docker compose`, `systemctl` or `curl` is
//! therefore **structurally faithful and live-unverified**. What IS verified:
//!
//! * the exact program + argument vector, in order, for every spawn — `tests`;
//! * the exact stdin payload for every `ssh … bash -s` heredoc — `tests`;
//! * the four-outcome exit-code read of `mod remote-logs` — `v6_verdict` + `tests`;
//! * the whole `--dry-run` walk, which is byte-diffed against the bash baseline and never opens a
//!   socket (see the note on [`Runner`]).
//!
//! What is NOT verified: whether a real `ssh` accepts these argv, whether the remote `bash -s`
//! payloads behave on the host, whether `docker compose` is reachable there, and whether the boot
//! wait loop's timing assumptions hold. Those were never true of the bash either — the bash was
//! only ever exercised by the operator running a live deploy — so this is a statement about the
//! test environment, not a regression.
//!
//! ── THE EXCLUDE LIST IS A LICENCE BOUNDARY, NOT AN OPTIMISATION ──────────────────────────────
//!
//! T-181.52: EXCLUDE EVERY ORACLE LANE, not just CRF. These are read-only reference trees; the
//! server only ever runs `apps/mod/tbd-framework` (see the addon symlink), so shipping them is
//! pure licence exposure for zero benefit. `crf_framework` was already excluded, but
//! `vanilla_reference` and `playable_selector` were NOT — and in the MAIN checkout (which is what
//! deploys) they are real directories, not the worktree symlinks, so ~30 MB of carved Bohemia game
//! source was being rsynced to staging on every deploy. `playable_selector` has NO LICENCE AT ALL,
//! so copying it to a server is redistribution we have no permission for. Anyone adding a fourth
//! oracle lane adds it here too.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use verification_core::proc::{self, Run};
use verification_core::verdict::NotRun;

use super::agent::{self, AgentEnv};
use super::boot::{self, Out};
use super::config::Env;
use super::payloads::{AGENT_INSTALL_PAYLOAD, profile_payload, smoke_payload, unit_payload};
use super::{Cli, Paths};

/// How `ssh` is invoked: plain, via `sshpass`, or with an identity file.
///
/// ODDITY PRESERVED: the precedence is `TBD_SSH_PASS` first, `TBD_SSH_IDENTITY_FILE` second — a
/// deploy.env holding both silently ignores the key. And in the `rsync -e` string the password is
/// interpolated UNQUOTED, so a password containing a space would split into extra argv words for
/// the inner ssh. Reproduced rather than fixed: the fix (quoting) changes what a working
/// configuration does, and no configuration with a spaced password can currently be working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshBase {
    Plain,
    Pass(String),
    Identity(String),
}

impl SshBase {
    pub fn from_env(env: &Env) -> SshBase {
        if let Some(p) = env.ssh_pass.as_deref() {
            SshBase::Pass(p.to_string())
        } else if let Some(i) = env.ssh_identity_file.as_deref() {
            SshBase::Identity(i.to_string())
        } else {
            SshBase::Plain
        }
    }

    /// `SSH_BASE=(...)` — the program and its leading arguments.
    pub fn program_args(&self) -> (String, Vec<String>) {
        match self {
            SshBase::Plain => (
                "ssh".into(),
                vec!["-o".into(), "StrictHostKeyChecking=no".into()],
            ),
            SshBase::Pass(p) => (
                "sshpass".into(),
                vec![
                    "-p".into(),
                    p.clone(),
                    "ssh".into(),
                    "-o".into(),
                    "StrictHostKeyChecking=no".into(),
                ],
            ),
            SshBase::Identity(i) => (
                "ssh".into(),
                vec![
                    "-i".into(),
                    i.clone(),
                    "-o".into(),
                    "StrictHostKeyChecking=no".into(),
                ],
            ),
        }
    }

    /// The `rsync -e <string>` transport. One shell word per space, unquoted, as in the bash.
    pub fn rsync_e(&self) -> String {
        match self {
            SshBase::Plain => "ssh -o StrictHostKeyChecking=no".into(),
            SshBase::Pass(p) => format!("sshpass -p {p} ssh -o StrictHostKeyChecking=no"),
            SshBase::Identity(i) => format!("ssh -i {i} -o StrictHostKeyChecking=no"),
        }
    }
}

/// The bash `run()` wrapper: echo under `--dry-run`, execute otherwise.
///
/// LATENT FINDING, reported not fixed: in the shell script this dry-run branch was **dead code**.
/// Every call site of `ssh_cmd` / `rsync_to_remote` already sat inside an explicit
/// `if [ "$DRY_RUN" -eq 1 ]` arm that printed a bespoke `[dry-run] …` line and skipped the call, so
/// `run`'s generic `echo "[dry-run] $*"` could never fire. It is kept — with the same shape — so
/// that a call site added later without its own guard still cannot reach the network. Removing it
/// would convert a latent no-op into a live `ssh`.
pub struct Runner {
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
        if let Err(e) = proc::which(&program) {
            return Err(not_run_exit(&e));
        }
        let mut run = Run::new(&program);
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

    /// `ssh_cmd "…"` capturing stdout, for the boot-verify probes.
    fn ssh_capture(
        &self,
        base: &SshBase,
        host: &str,
        remote: &[String],
    ) -> Result<(i32, String), u8> {
        let argv = ssh_argv(base, host, remote);
        if self.dry_run {
            println!("[dry-run] {}", argv.join(" "));
            return Ok((0, String::new()));
        }
        let (program, _) = base.program_args();
        if let Err(e) = proc::which(&program) {
            return Err(not_run_exit(&e));
        }
        let mut run = Run::new(&program);
        for a in argv.iter().skip(1) {
            run = run.arg(a);
        }
        match run.timeout(Duration::from_secs(600)).output() {
            Ok(out) => Ok((out.code, out.stdout)),
            Err(e) => Err(not_run_exit(&e)),
        }
    }
}

#[cfg(test)]
#[path = "tests/remote/tests.rs"]
mod tests;

mod ssh_argv;
pub use ssh_argv::deploy;
use ssh_argv::not_run_exit;
pub use ssh_argv::ssh_argv;

mod verify_boot_remote;
use verify_boot_remote::verify_boot_remote;

#[cfg(test)]
pub(crate) use ssh_argv::{exec_start, rsync_argv, v6_verdict};
