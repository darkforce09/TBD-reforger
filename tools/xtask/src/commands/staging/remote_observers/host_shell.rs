//! The live transport: one ssh process per remote command.
//!
//! **Role:** runs a [`RemoteCommand`] on the staging host over the shared ssh transport and
//! returns its exit code and standard output.
//!
//! **Position:** the [`HostCommandRunner`] of every live harness command; built from
//! [`StagingSettings`]; the argv comes from [`process_runner::secure_shell_transport`].
//!
//! **Signals & state:** holds the ssh base and destination; each call spawns one process.
//!
//! **Invariants:** the ssh password reaches `sshpass -e` through the child's environment only;
//! the command line is the only remote argument, so the host's shell parses exactly what the
//! builder quoted; a missing `ssh` or `sshpass` is an error, never an empty observation.

use std::time::Duration;

use anyhow::{Result, anyhow};
use process_runner::Run;

use super::remote_command::{CommandOutput, CommandPurpose, HostCommandRunner, RemoteCommand};
use crate::commands::staging::staging_settings::StagingSettings;
use process_runner::secure_shell_transport::{SshBase, ssh_argv};

/// How long one read may take before it counts as not run.
const READ_TIMEOUT: Duration = Duration::from_secs(120);
/// How long one host action may take (a steamcmd validate runs for minutes).
const CHANGE_TIMEOUT: Duration = Duration::from_secs(3600);

/// ssh to the staging host.
#[derive(Debug, Clone)]
pub(crate) struct HostShell {
    ssh: SshBase,
    destination: String,
}

impl HostShell {
    /// The shell of the host `settings` name.
    pub(crate) fn new(settings: &StagingSettings) -> Self {
        Self {
            ssh: settings.ssh.clone(),
            destination: settings.host.ssh_destination(),
        }
    }

    /// The full argv of `command`, program first.
    pub(crate) fn argv(&self, command: &RemoteCommand) -> Vec<String> {
        ssh_argv(
            &self.ssh,
            &self.destination,
            std::slice::from_ref(&command.command_line),
        )
    }
}

impl HostCommandRunner for HostShell {
    fn run(&mut self, command: &RemoteCommand) -> Result<CommandOutput> {
        let argv = self.argv(command);
        process_runner::which(&argv[0]).map_err(|error| anyhow!("{error:?}"))?;
        let mut run = self.ssh.with_password(Run::new(&argv[0]));
        for argument in &argv[1..] {
            run = run.arg(argument);
        }
        if let Some(stdin) = &command.stdin {
            run = run.stdin(stdin.clone());
        }
        let timeout = match command.purpose {
            CommandPurpose::Read => READ_TIMEOUT,
            CommandPurpose::Change => CHANGE_TIMEOUT,
        };
        let output = run
            .timeout(timeout)
            .output()
            .map_err(|error| anyhow!("{} did not run: {error:?}", command.observer))?;
        Ok(CommandOutput {
            exit_code: output.code,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}
