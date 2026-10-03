//! The ssh transport: how `ssh` is invoked and the argument vector of one remote command.
//!
//! **Role:** decides how `ssh` is invoked (plain, through `sshpass -e`, or with an identity file)
//! and builds the full argument vector of one remote command.
//!
//! **Position:** the caller chooses the form from its own settings with
//! [`SshBase::from_settings`] (xtask reads them from `deploy.env`); the staging deploy and the
//! staging harness of xtask spawn the argv this module builds through [`crate::Run`].
//!
//! **Signals & state:** none; pure values and functions.
//!
//! **Invariants:** no argument vector and no `Debug` rendering carries the ssh password:
//! `sshpass -e` reads it from [`SSH_PASSWORD_VARIABLE`] in the spawned process's environment
//! only. A password wins over an identity file, so settings holding both ignore the key.

use std::fmt;

use crate::Run;

/// The variable `sshpass -e` reads the password from.
pub const SSH_PASSWORD_VARIABLE: &str = "SSHPASS";

/// How `ssh` is invoked: plain, via `sshpass`, or with an identity file.
#[derive(Clone, PartialEq, Eq)]
pub enum SshBase {
    /// Plain `ssh`, authenticating with the agent or the default keys.
    Plain,
    /// `sshpass -e ssh`, with this password handed over in the child's environment only.
    Pass(String),
    /// `ssh -i <file>`, with this identity file.
    Identity(String),
}

impl fmt::Debug for SshBase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plain => formatter.write_str("Plain"),
            Self::Pass(_) => formatter.write_str("Pass(<redacted>)"),
            Self::Identity(file) => formatter.debug_tuple("Identity").field(file).finish(),
        }
    }
}

impl SshBase {
    /// The password first, the identity file second, else plain `ssh`.
    pub fn from_settings(password: Option<&str>, identity_file: Option<&str>) -> SshBase {
        match (password, identity_file) {
            (Some(password), _) => SshBase::Pass(password.to_string()),
            (None, Some(identity_file)) => SshBase::Identity(identity_file.to_string()),
            (None, None) => SshBase::Plain,
        }
    }

    /// The program and its leading arguments.
    pub fn program_args(&self) -> (String, Vec<String>) {
        let strict = ["-o".to_string(), "StrictHostKeyChecking=no".to_string()];
        match self {
            SshBase::Plain => ("ssh".into(), strict.to_vec()),
            SshBase::Pass(_) => {
                let mut arguments = vec!["-e".to_string(), "ssh".to_string()];
                arguments.extend(strict);
                ("sshpass".into(), arguments)
            }
            SshBase::Identity(identity_file) => {
                let mut arguments = vec!["-i".to_string(), identity_file.clone()];
                arguments.extend(strict);
                ("ssh".into(), arguments)
            }
        }
    }

    /// The `rsync -e <string>` transport: one shell word per space, unquoted.
    pub fn rsync_e(&self) -> String {
        match self {
            SshBase::Plain => "ssh -o StrictHostKeyChecking=no".into(),
            SshBase::Pass(_) => "sshpass -e ssh -o StrictHostKeyChecking=no".into(),
            SshBase::Identity(i) => format!("ssh -i {i} -o StrictHostKeyChecking=no"),
        }
    }

    /// The password `sshpass -e` reads from [`SSH_PASSWORD_VARIABLE`], for the spawned process's
    /// environment only.
    pub fn password(&self) -> Option<&str> {
        match self {
            SshBase::Pass(p) => Some(p),
            _ => None,
        }
    }

    /// `run` with the password in its environment when there is one.
    pub fn with_password(&self, run: Run) -> Run {
        match self.password() {
            Some(p) => run.env(SSH_PASSWORD_VARIABLE, p),
            None => run,
        }
    }
}

/// The full `ssh` argv, program included at index 0. Pure, so it can be asserted without spawning.
pub fn ssh_argv(base: &SshBase, host: &str, remote: &[String]) -> Vec<String> {
    let (program, mut args) = base.program_args();
    let mut argv = vec![program];
    argv.append(&mut args);
    argv.push(host.to_string());
    argv.extend(remote.iter().cloned());
    argv
}

#[cfg(test)]
#[path = "tests/secure_shell_transport_tests.rs"]
mod tests;
