//! `cargo xtask staging preflight [--discord]`: every precondition of the procedures, checked
//! without changing anything.
//!
//! **Role:** the [`PreflightCheck`] value procedures supply, the harness's own checks (run
//! discipline, the database's read-only session, the API's health, the fleet units, the host
//! tools), and the runner that prints each check as met or unmet.
//!
//! **Position:** `staging_dispatch.rs` runs the harness checks, then the fleet and load procedures'
//! checks, and the Discord procedure's with `--discord`; host checks run through
//! `remote_observers/host_shell.rs`.
//!
//! **Signals & state:** none; one read per host check.
//!
//! **Invariants:** a check list holding a host command that is not a read is refused before any
//! check runs, so preflight never changes the host; an unmet check names its reason and makes the
//! exit code 1.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{Result, ensure};

use crate::remote_actions::host_fixture_commands::HOST_TOOL;
use crate::remote_actions::relay_control::RELAY_BINARY;
use crate::remote_observers::database_reader::{self, SESSION_GUARD};
use crate::remote_observers::remote_command::{
    CommandOutput, CommandPurpose, HostCommandRunner, RemoteCommand, shell_quote,
};
use crate::remote_observers::unit_state_reader;
use crate::staging_settings::{STAGING_DATABASE, StagingSettings};

/// Judges a host check's answer: `Ok(evidence)` when met, `Err(reason)` when not.
pub(crate) type HostJudge = Box<dyn Fn(&CommandOutput) -> Result<String, String>>;
/// Judges a local check: `Ok(evidence)` when met, `Err(reason)` when not.
pub(crate) type LocalJudge = Box<dyn Fn() -> Result<String, String>>;

/// Where a check looks.
pub(crate) enum PreflightProbe {
    /// A read of the staging host.
    Host {
        command: RemoteCommand,
        judge: HostJudge,
    },
    /// A look at this workstation.
    Local(LocalJudge),
}

/// One named precondition.
pub(crate) struct PreflightCheck {
    pub name: String,
    pub probe: PreflightProbe,
}

impl PreflightCheck {
    /// A precondition read on the host.
    pub(crate) fn host(
        name: impl Into<String>,
        command: RemoteCommand,
        judge: impl Fn(&CommandOutput) -> Result<String, String> + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            probe: PreflightProbe::Host {
                command,
                judge: Box::new(judge),
            },
        }
    }

    /// A precondition of this workstation.
    pub(crate) fn local(
        name: impl Into<String>,
        judge: impl Fn() -> Result<String, String> + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            probe: PreflightProbe::Local(Box::new(judge)),
        }
    }
}

/// The checks every procedure shares.
pub(crate) fn harness_checks(settings: &StagingSettings, root: &Path) -> Vec<PreflightCheck> {
    let root = root.to_path_buf();
    let mut checks = vec![PreflightCheck::local("repository root", move || {
        let here = std::env::current_dir().map_err(|error| error.to_string())?;
        match same_folder(&here, &root) {
            true => Ok(format!("running from {}", root.display())),
            false => Err(format!(
                "run from {}, not {}",
                root.display(),
                here.display()
            )),
        }
    })];
    if let Ok(command) = database_reader::select(&settings.database_container, &SESSION_GUARD, &[])
    {
        checks.push(PreflightCheck::host(
            "database read-only session",
            command,
            |output| {
                let expected = format!("{STAGING_DATABASE}|on");
                match output.stdout.trim() == expected {
                    true if output.exit_code == 0 => Ok(expected),
                    _ => Err(format!(
                        "expected {expected}, got {:?} (exit {})",
                        output.stdout.trim(),
                        output.exit_code
                    )),
                }
            },
        ));
    }
    let health = shell_quote(&format!("{}/healthz", settings.api_origin));
    checks.push(PreflightCheck::host(
        "API health",
        RemoteCommand::read(
            "api health",
            format!("curl -sS -o /dev/null -w '%{{http_code}}' --max-time 10 {health}"),
        ),
        |output| match output.stdout.trim() {
            "200" => Ok("/healthz answered 200".to_string()),
            other => Err(format!(
                "/healthz answered {other:?} (exit {})",
                output.exit_code
            )),
        },
    ));
    let mut units = settings.game_server_units();
    units.extend(settings.host_agent_units());
    units.extend(settings.relay_unit().map(|(_, unit)| unit));
    let expected = units.clone();
    checks.push(PreflightCheck::host(
        "fleet units",
        unit_state_reader::show(&units),
        move |output| {
            let states = unit_state_reader::parse(&output.stdout);
            let inactive: Vec<String> = expected
                .iter()
                .filter(|unit| {
                    states
                        .get(*unit)
                        .is_none_or(|state| state.active_state != "active")
                })
                .cloned()
                .collect();
            match inactive.is_empty() {
                true => Ok(format!("{} units active", expected.len())),
                false => Err(format!("not active: {}", inactive.join(", "))),
            }
        },
    ));
    let tool = shell_quote(&format!("{}/{HOST_TOOL}", settings.checkout));
    checks.push(PreflightCheck::host(
        "host tools",
        RemoteCommand::read(
            "host tools",
            format!(
                "for tool in {tool} \"{RELAY_BINARY}\"; do if [ -x \"$tool\" ]; then echo \"present $tool\"; else echo \"missing $tool\"; fi; done"
            ),
        ),
        |output| {
            let missing: Vec<&str> = output.stdout.lines().filter(|line| line.starts_with("missing")).collect();
            match (missing.is_empty(), output.exit_code) {
                (true, 0) => Ok("staging-fixtures and acknowledgement-dropping-relay present".to_string()),
                _ => Err(format!("{} (exit {})", missing.join("; "), output.exit_code)),
            }
        },
    ));
    checks
}

/// Runs `checks` in order and prints each; exit 0 when every check is met, else 1.
pub(crate) fn run(
    checks: &[PreflightCheck],
    host: &mut dyn HostCommandRunner,
    output: &mut dyn Write,
) -> Result<u8> {
    for check in checks {
        if let PreflightProbe::Host { command, .. } = &check.probe {
            ensure!(
                command.purpose == CommandPurpose::Read,
                "preflight check {:?} would change the host; preflight only reads",
                check.name
            );
        }
    }
    let mut unmet = 0usize;
    for check in checks {
        let result = match &check.probe {
            PreflightProbe::Host { command, judge } => match host.run(command) {
                Ok(answer) => judge(&answer),
                Err(error) => Err(format!("{error:#}")),
            },
            PreflightProbe::Local(judge) => judge(),
        };
        match result {
            Ok(evidence) => writeln!(output, "met    {}: {evidence}", check.name)?,
            Err(reason) => {
                unmet += 1;
                writeln!(output, "UNMET  {}: {reason}", check.name)?;
            }
        }
    }
    writeln!(
        output,
        "preflight: {} met, {unmet} unmet",
        checks.len() - unmet
    )?;
    Ok(u8::from(unmet > 0))
}

fn same_folder(left: &Path, right: &Path) -> bool {
    let canonical =
        |path: &Path| -> PathBuf { path.canonicalize().unwrap_or_else(|_| path.to_path_buf()) };
    canonical(left) == canonical(right)
}
