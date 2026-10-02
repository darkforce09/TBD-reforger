//! `cargo xtask mod bootstrap-staging` — one-time discovery of a staging host, and the folders
//! the deploy expects to find there.
//!
//! **Role:** over ssh, prints the host's disk, the listeners on 5432, 8080 and 2001 and the
//! container runtime, creates the checkout, profile and addon folders, and prints the manual next
//! steps. It installs neither steamcmd nor Arma; the staging runbook covers those.
//!
//! **Position:** reached from `cargo xtask mod bootstrap-staging`
//! (`crate::commands::mod_ops::dispatch`). The host and the folders come from `deploy.env`
//! through [`crate::core::deploy_environment`], under its precedence rule; an absent file is
//! allowed, and then every value comes from the process environment.
//!
//! **Signals & state:** none beyond the ssh calls. Two test seams, preferred over `PATH` stubs
//! because `PATH` is process-wide: `TBD_BOOTSTRAP_STAGING_SSH` and
//! `TBD_BOOTSTRAP_STAGING_SSHPASS` name an absolute ssh or sshpass; set but missing, they force
//! [`NotRun::ToolAbsent`].
//!
//! **Invariants:** exit 1 for a deploy file that does not load, a missing or malformed
//! `TBD_SSH_HOST`, a folder that cannot be resolved, and a `TBD_REMOTE_DIR` containing
//! `prairielearn` (the neighbouring project on the same host; the match is case-sensitive);
//! exit 127 with an explicit message when `ssh` or `sshpass` is absent, never a silent success.
//! Transport failures and non-zero remote exits stop the command. The discovery script's own
//! probes are soft on purpose: it reports what a host has, and an absent tool is an answer.

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;
use verification_core::proc::{self, Run};
use verification_core::verdict::NotRun;

use crate::core::deploy_environment::{
    DeployEnvironment, DeployHostFolder, SettingError, deploy_environment_path,
};
use crate::core::repository_root::find_repo_root;

/// Optional absolute ssh path for unit tests (avoids PATH mutation).
const ENV_SSH: &str = "TBD_BOOTSTRAP_STAGING_SSH";
/// Optional absolute sshpass path for unit tests (avoids PATH mutation).
const ENV_SSHPASS: &str = "TBD_BOOTSTRAP_STAGING_SSHPASS";

/// The discovery script run on the remote host: disk, listening ports, container runtime.
const DISCOVERY_SCRIPT: &str = r#"set -euo pipefail
echo "--- disk ---"
df -h ~
echo "--- ports 5432 8080 2001 ---"
ss -tlnp 2>/dev/null | grep -E ':5432|:8080|:2001' || echo "(none listening on those TCP ports)"
echo "--- docker ---"
docker compose version 2>/dev/null || docker --version 2>/dev/null || echo "docker not found"
"#;

/// Entry for `xtask mod bootstrap-staging`.
pub fn run() -> Result<u8> {
    let root = find_repo_root()?;
    match DeployEnvironment::load_if_present(&deploy_environment_path(&root)) {
        Ok(environment) => run_with_environment(&environment),
        Err(error) => {
            eprintln!("{error}");
            Ok(1)
        }
    }
}

/// Testable entry over deploy settings the caller built.
pub fn run_with_environment(environment: &DeployEnvironment) -> Result<u8> {
    let cfg = match load_cfg(environment) {
        Ok(c) => c,
        Err(error) => {
            eprintln!("{error}");
            return Ok(1);
        }
    };

    if cfg.remote_dir.contains("prairielearn") {
        eprintln!("Refusing: TBD_REMOTE_DIR must not be under prairielearn/");
        return Ok(1);
    }

    println!("==> Discovery on {}", cfg.host);
    if let Err(code) = ssh_bash_s(&cfg, DISCOVERY_SCRIPT) {
        return Ok(code);
    }

    println!("==> Create TBD directories (not prairielearn)");
    let mkdir_script = format!(
        "set -euo pipefail\nmkdir -p \"{}\" \"{}\" \"{}\"\necho \"OK: {} {} {}\"\n",
        cfg.remote_dir,
        cfg.profile_dir,
        cfg.addons_staging,
        cfg.remote_dir,
        cfg.profile_dir,
        cfg.addons_staging,
    );
    if let Err(code) = ssh_bash_s(&cfg, &mkdir_script) {
        return Ok(code);
    }

    println!();
    for line in next_steps() {
        println!("{line}");
    }

    Ok(0)
}

/// The manual steps after discovery. The fleet's secrets are files on the host, one set per
/// instance, which `cargo xtask deploy staging` checks before it changes anything; the deploy
/// generates each instance's RCON password itself.
fn next_steps() -> Vec<String> {
    vec![
        format!(
            "Next steps (manual — see {}):",
            crate::core::repository_layout::documentation::STAGING_SERVER_RUNBOOK
        ),
        "  1. steamcmd +app_update 1890870 on server".into(),
        "  2. Create apps/website/api_v2/.env on server (JWT_SECRET + OBSERVABILITY_TOKEN)".into(),
        "  3. sudo loginctl enable-linger \"$USER\"   (on the host, as the deploy user)".into(),
        "  4. Register one game server per fleet instance N in Server Control, issue each its".into(),
        "     mod_runtime and host_agent credentials, and write them on the host to".into(),
        "     ~/tbd/fleet/instance-N/secrets/mod-runtime-credential and host-agent-credential".into(),
        "     (folders mode 700, files mode 600); put the join password in ~/tbd/fleet/join-password"
            .into(),
        "  5. cargo xtask deploy staging".into(),
    ]
}

struct Cfg {
    /// The ssh destination, `user@host` or `host`.
    host: String,
    remote_dir: String,
    profile_dir: String,
    addons_staging: String,
    ssh_pass: Option<String>,
    ssh_identity: Option<String>,
}

fn load_cfg(environment: &DeployEnvironment) -> Result<Cfg, SettingError> {
    let host = environment.deploy_host()?;
    let folder = |folder: DeployHostFolder| folder.resolve(environment, &host);
    Ok(Cfg {
        remote_dir: folder(DeployHostFolder::Checkout)?,
        profile_dir: folder(DeployHostFolder::Profile)?,
        addons_staging: folder(DeployHostFolder::AddonsStaging)?,
        host: host.ssh_destination(),
        ssh_pass: environment.value("TBD_SSH_PASS").map(str::to_string),
        ssh_identity: environment
            .value("TBD_SSH_IDENTITY_FILE")
            .map(str::to_string),
    })
}

fn ssh_bash_s(cfg: &Cfg, script: &str) -> Result<(), u8> {
    let (program, mut args) = ssh_base(cfg)?;
    args.push(cfg.host.clone());
    args.push("bash".into());
    args.push("-s".into());

    let mut run = Run::new(&program).stdin(script);
    for a in &args {
        run = run.arg(a);
    }
    match run.merged_output() {
        Ok(out) => {
            let _ = io::stdout().write_all(out.text.as_bytes());
            if out.code == 0 {
                Ok(())
            } else {
                Err(out.code as u8)
            }
        }
        Err(e) => Err(not_run_exit(&e)),
    }
}

fn resolve_tool(env_key: &str, name: &str) -> Result<PathBuf, NotRun> {
    if let Ok(override_path) = std::env::var(env_key) {
        let trimmed = override_path.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.is_file() {
                return Ok(p);
            }
            return Err(NotRun::ToolAbsent(name.to_string()));
        }
    }
    proc::which(name)
}

fn ssh_base(cfg: &Cfg) -> Result<(String, Vec<String>), u8> {
    if let Some(ref pass) = cfg.ssh_pass {
        // Closed fail-open: absent sshpass/ssh is NotRun, not a silent success.
        let sshpass = match resolve_tool(ENV_SSHPASS, "sshpass") {
            Ok(p) => p,
            Err(e) => return Err(not_run_exit(&e)),
        };
        let ssh = match resolve_tool(ENV_SSH, "ssh") {
            Ok(p) => p,
            Err(e) => return Err(not_run_exit(&e)),
        };
        Ok((
            sshpass.display().to_string(),
            vec![
                "-p".into(),
                pass.clone(),
                ssh.display().to_string(),
                "-o".into(),
                "StrictHostKeyChecking=no".into(),
            ],
        ))
    } else if let Some(ref id) = cfg.ssh_identity {
        let ssh = match resolve_tool(ENV_SSH, "ssh") {
            Ok(p) => p,
            Err(e) => return Err(not_run_exit(&e)),
        };
        Ok((
            ssh.display().to_string(),
            vec![
                "-i".into(),
                id.clone(),
                "-o".into(),
                "StrictHostKeyChecking=no".into(),
            ],
        ))
    } else {
        let ssh = match resolve_tool(ENV_SSH, "ssh") {
            Ok(p) => p,
            Err(e) => return Err(not_run_exit(&e)),
        };
        Ok((
            ssh.display().to_string(),
            vec!["-o".into(), "StrictHostKeyChecking=no".into()],
        ))
    }
}

fn not_run_exit(e: &NotRun) -> u8 {
    // An absent tool is reported as itself; it never folds into a zero exit code.
    match e {
        NotRun::ToolAbsent(tool) => {
            eprintln!("{tool}: command not found");
            127
        }
        other => {
            eprintln!("{other:?}");
            1
        }
    }
}

#[cfg(test)]
#[path = "tests/staging_server/tests.rs"]
mod tests;
