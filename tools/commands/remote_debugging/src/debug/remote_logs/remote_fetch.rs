//! The remote half of `mod remote-logs`: whose `console.log` to read, the script that finds the
//! newest one on the staging host, and the ssh fetch.
//!
//! **Role:** resolves the host, the ssh credentials and the [`LogSource`] from `deploy.env` and
//! `--instance`, fetches the newest `console.log` and hands a local copy to the verdict.
//!
//! **Position:** called by `run` in [`super::execution`]; the fleet instance, its profile folder
//! and the fleet folder come from `cargo xtask deploy staging`'s own fleet rules
//! ([`deployment::staging::fleet_instances`]) through
//! [`crate::debug::staging_fleet_instance`].
//!
//! **Signals & state:** none held; one run spawns two ssh children and writes one temp file,
//! removed before it returns.
//!
//! **Invariants:** `--instance N` reads `~/tbd/fleet/instance-N/profile`; without it the single
//! server's profile folder is read only while the host holds no `~/tbd/fleet`, and a host that
//! does is ENVIRONMENT with a message naming `--instance`; an instance outside the fleet is
//! ENVIRONMENT; a refused setting exits 1.

use super::execution::{check_log, env_fail};
use super::*;
use crate::debug::staging_fleet_instance::{
    InstanceSelectionError, profile_under_home, select_fleet_instance,
};
use deploy_settings::{DeployEnvironment, DeployHostFolder, deploy_environment_path};
use deployment::staging::fleet_instances::{
    FLEET_ROOT_UNDER_HOME, FleetInstance, MAXIMUM_FLEET_INSTANCES,
};

/// The newest-log script's exit when no instance was named and the host runs a fleet.
pub(super) const FLEET_HOST_EXIT: i32 = 10;

/// Whose `console.log` is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LogSource {
    /// `--instance N`: that fleet instance's profile under the deploy user's home.
    FleetInstance(FleetInstance),
    /// No `--instance`: the single server's profile folder, `TBD_PROFILE_DIR`.
    SingleServer(String),
}

impl LogSource {
    /// The profile folder as one shell word; a fleet profile is anchored at `"$HOME"`.
    fn profile_word(&self) -> String {
        match self {
            Self::FleetInstance(instance) => {
                format!("\"$HOME\"/{}", shell_quote(&profile_under_home(instance)))
            }
            Self::SingleServer(profile) => shell_quote(profile),
        }
    }

    /// The profile folder as the messages name it.
    fn profile_label(&self) -> String {
        match self {
            Self::FleetInstance(instance) => format!("~/{}", profile_under_home(instance)),
            Self::SingleServer(profile) => profile.clone(),
        }
    }
}

/// Where `cmd_remote` reads the log from.
struct RemoteTarget {
    destination: String,
    source: LogSource,
    ssh_pass: Option<String>,
    ssh_identity: Option<String>,
}

/// The staging host, the log source and the ssh credentials, from `deploy.env` and `--instance`;
/// a refusal is printed and answered with its exit code.
fn remote_target(instance: Option<u16>) -> Result<Result<RemoteTarget, u8>> {
    let refused = |error: &dyn std::fmt::Display| {
        eprintln!("{error}");
        Ok(Err(1))
    };
    let path = deploy_environment_path(&find_repository_root()?);
    let environment = match DeployEnvironment::load_if_present(&path) {
        Ok(environment) => environment,
        Err(error) => return refused(&error),
    };
    let host = match environment.deploy_host() {
        Ok(host) => host,
        Err(error) => return refused(&error),
    };
    let source = match instance {
        Some(number) => match select_fleet_instance(&environment, number) {
            Ok(instance) => LogSource::FleetInstance(instance),
            Err(InstanceSelectionError::Setting(error)) => return refused(&error),
            Err(out_of_range) => return Ok(Err(env_fail(&out_of_range.to_string()))),
        },
        None => match DeployHostFolder::Profile.resolve(&environment, &host) {
            Ok(profile) => LogSource::SingleServer(profile),
            Err(error) => return refused(&error),
        },
    };
    Ok(Ok(RemoteTarget {
        destination: host.ssh_destination(),
        source,
        ssh_pass: environment.value("TBD_SSH_PASS").map(str::to_string),
        ssh_identity: environment
            .value("TBD_SSH_IDENTITY_FILE")
            .map(str::to_string),
    }))
}

/// The remote script that prints the newest run's `console.log` under the source's profile (its
/// `logs/` or its `profile/logs/`) and exits 0, or exits 1 when no run has one. For the single
/// server it first exits [`FLEET_HOST_EXIT`] when the host holds the fleet folder. The loop reads
/// `ls` through process substitution, so its `exit 0` ends the script: behind a pipe the loop
/// would run in a subshell, its `exit 0` would leave only that subshell, and the script would
/// always reach `exit 1`.
pub(super) fn newest_console_log_script(source: &LogSource) -> String {
    let profile = source.profile_word();
    let fleet_guard = match source {
        LogSource::SingleServer(_) => {
            format!("[ -e \"$HOME\"/{FLEET_ROOT_UNDER_HOME} ] && exit {FLEET_HOST_EXIT}\n")
        }
        LogSource::FleetInstance(_) => String::new(),
    };
    format!(
        "\n{fleet_guard}while read -r d; do\n  [ -f \"$d/console.log\" ] && echo \"$d/console.log\" && exit 0\ndone < <(ls -td {profile}/logs/logs_* {profile}/profile/logs/logs_* 2>/dev/null)\nexit 1\n"
    )
}

/// Fetches the newest `console.log` of the chosen server and grades it.
pub(super) fn cmd_remote(instance: Option<u16>) -> Result<u8> {
    // A refused setting exits 1, not ENVIRONMENT 3: nothing was probed.
    let RemoteTarget {
        destination: host,
        source,
        ssh_pass,
        ssh_identity: ssh_ident,
    } = match remote_target(instance)? {
        Ok(target) => target,
        Err(code) => return Ok(code),
    };
    let profile = source.profile_label();
    let find_log = newest_console_log_script(&source);

    let remote_log = match ssh_cmd(
        &host,
        ssh_pass.as_deref(),
        ssh_ident.as_deref(),
        &["bash", "-lc", &shell_quote(&find_log)],
    ) {
        Ok(out) if out.code == 0 => out.stdout.trim().to_string(),
        Ok(out) if out.code == FLEET_HOST_EXIT && matches!(source, LogSource::SingleServer(_)) => {
            return Ok(env_fail(&format!(
                "{host} runs a fleet under ~/{FLEET_ROOT_UNDER_HOME}; pass --instance N \
                 (1 to {MAXIMUM_FLEET_INSTANCES}) to read that instance's log"
            )));
        }
        // Any other ssh failure leaves no log; ENVIRONMENT below, never a log verdict.
        _ => String::new(),
    };

    if remote_log.is_empty() {
        return Ok(env_fail(&format!(
            "no console.log found under {profile} (logs/ or profile/logs/) on {host}"
        )));
    }

    let local_copy = std::env::temp_dir().join(format!("tbd-remote-log.{}", std::process::id()));
    let cat = ssh_cmd(
        &host,
        ssh_pass.as_deref(),
        ssh_ident.as_deref(),
        &["cat", &remote_log],
    );
    match cat {
        Ok(out) if out.code == 0 => {
            if out.stdout.is_empty() {
                let _ = fs::remove_file(&local_copy);
                return Ok(env_fail(&format!("{remote_log} on {host} is empty")));
            }
            fs::write(&local_copy, &out.stdout).context("write local log copy")?;
        }
        _ => {
            let _ = fs::remove_file(&local_copy);
            return Ok(env_fail(&format!(
                "could not read {remote_log} from {host}"
            )));
        }
    }

    println!("Remote log: {host}:{remote_log}");
    let rc = check_log(&local_copy);
    let _ = fs::remove_file(&local_copy);
    Ok(rc)
}

/// One ssh call: sshpass with a password, `-i` with an identity file, plain ssh otherwise. An
/// absent tool or a signal is [`NotRun`], never a verdict.
pub(super) fn ssh_cmd(
    host: &str,
    pass: Option<&str>,
    ident: Option<&str>,
    remote_args: &[&str],
) -> Result<SshOut, NotRun> {
    let mut args: Vec<String> = Vec::new();
    let program = match (
        pass.filter(|s| !s.is_empty()),
        ident.filter(|s| !s.is_empty()),
    ) {
        (Some(p), _) => {
            args.extend(["-p".into(), p.into(), "ssh".into()]);
            "sshpass"
        }
        (None, Some(id)) => {
            args.extend(["-i".into(), id.into()]);
            "ssh"
        }
        (None, None) => "ssh",
    };
    args.extend(["-o".into(), "StrictHostKeyChecking=no".into(), host.into()]);
    args.extend(remote_args.iter().map(|a| (*a).to_string()));

    let _ = process_runner::which(program)?;
    let mut run = Run::new(program);
    for a in &args {
        run = run.arg(a);
    }
    let out = run.output()?;
    Ok(SshOut {
        code: out.code,
        stdout: out.stdout,
    })
}
