//! The deploy settings file, `deploy.env`, read the same way by every command that reaches a host.
//!
//! **Role:** loads `deploy.env` and answers each setting under one precedence rule: the file decides
//! every key it assigns, an empty assignment counting as unset, and the process environment fills
//! only the keys the file never assigns. An explicit command-line flag, where a command has one,
//! beats both; that choice stays with the command.
//!
//! **Position:** the loader of the `deploy_settings` crate, read by the `deploy`, `setup`,
//! `debug`, `mod` and `staging` command groups. [`deploy_environment_path`] picks the file (the
//! `DEPLOY_ENV` variable, else [`repository_layout::DEPLOY_ENV`] in the checkout);
//! [`crate::assignment_syntax`] owns the file's grammar, [`DeployHost`] the value of
//! `TBD_SSH_HOST`, [`crate::DeployHostFolder`] the remote folders that default under the deploy
//! user's home, and [`DeployEnvironment::ssh_base`] the ssh transport `process_runner` builds the
//! remote argv with; the errors are in [`crate::error`].
//!
//! **Signals & state:** a [`DeployEnvironment`] is an immutable snapshot of the file and of the
//! process environment, taken when it is loaded; nothing here writes either.
//!
//! **Invariants:** the file is parsed and never executed; a stale exported variable never
//! overrides a key the file assigns; a present but unusable value is reported as `<path>:<line>`,
//! and a missing one names the file.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use process_runner::secure_shell_transport::SshBase;

use crate::assignment_syntax;
use crate::deploy_host::DeployHost;
use crate::error::{Error, SettingError, SettingOrigin};

/// The process variable that points every command at another deploy settings file.
pub const DEPLOY_ENV_OVERRIDE_VARIABLE: &str = "DEPLOY_ENV";

/// The setting that names the deploy host, as `user@host` or `host`.
pub const DEPLOY_HOST_KEY: &str = "TBD_SSH_HOST";

/// The setting holding the ssh password, when the deploy host takes one.
pub const SSH_PASSWORD_KEY: &str = "TBD_SSH_PASS";

/// The setting naming the ssh identity file, when the deploy host takes a key.
pub const SSH_IDENTITY_FILE_KEY: &str = "TBD_SSH_IDENTITY_FILE";

/// The deploy settings file this process reads, resolved by
/// [`resolve_deploy_environment_path`] from `DEPLOY_ENV` and the working directory.
pub fn deploy_environment_path(repository_root: &Path) -> PathBuf {
    let override_value = std::env::var_os(DEPLOY_ENV_OVERRIDE_VARIABLE);
    let working_directory =
        std::env::current_dir().unwrap_or_else(|_| repository_root.to_path_buf());
    resolve_deploy_environment_path(
        repository_root,
        override_value.as_deref(),
        &working_directory,
    )
}

/// A non-empty override, made absolute against `working_directory`; otherwise
/// [`repository_layout::DEPLOY_ENV`] under `repository_root`.
///
/// The answer is absolute whenever the root is, so a child process handed it reads the same file
/// from any working directory.
pub fn resolve_deploy_environment_path(
    repository_root: &Path,
    override_value: Option<&OsStr>,
    working_directory: &Path,
) -> PathBuf {
    match override_value.filter(|value| !value.is_empty()) {
        Some(value) => working_directory.join(value),
        None => repository_root.join(repository_layout::DEPLOY_ENV),
    }
}

/// The last assignment of one key in the file.
#[derive(Debug, Clone)]
struct FileValue {
    value: String,
    line: usize,
}

/// The deploy settings of one command run: the file's assignments and the process environment,
/// answered under the precedence rule of this module.
#[derive(Debug, Clone)]
pub struct DeployEnvironment {
    path: PathBuf,
    file_values: HashMap<String, FileValue>,
    process_values: HashMap<String, String>,
}

impl DeployEnvironment {
    /// Loads a file that must exist, for the commands that change a host.
    pub fn load_required(path: &Path) -> Result<Self, Error> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_text(path, Some(&text), process_snapshot()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Err(Error::Missing {
                path: path.to_path_buf(),
            }),
            Err(error) => Err(Error::Unreadable {
                path: path.to_path_buf(),
                error,
            }),
        }
    }

    /// Loads the file when it exists; without it every value comes from the process environment.
    pub fn load_if_present(path: &Path) -> Result<Self, Error> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(Error::Unreadable {
                    path: path.to_path_buf(),
                    error,
                });
            }
        };
        Self::from_text(path, text.as_deref(), process_snapshot())
    }

    /// Builds the settings from the file's text (`None` when there is no file) and the given
    /// process variables. The loaders call it with the real file and environment; tests call it
    /// with their own.
    pub fn from_text(
        path: &Path,
        file_text: Option<&str>,
        process: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, Error> {
        let assignments = match file_text {
            Some(text) => {
                assignment_syntax::parse_assignments(text).map_err(|(line, message)| {
                    Error::Syntax {
                        path: path.to_path_buf(),
                        line,
                        message,
                    }
                })?
            }
            None => Vec::new(),
        };
        let mut file_values = HashMap::new();
        for assignment in assignments {
            // A repeated key keeps its last assignment, as a shell would.
            file_values.insert(
                assignment.key,
                FileValue {
                    value: assignment.value,
                    line: assignment.line,
                },
            );
        }
        Ok(Self {
            path: path.to_path_buf(),
            file_values,
            process_values: process.into_iter().collect(),
        })
    }

    /// The settings file these values were read from, present or not.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The key's non-empty value: the file's when it assigns the key, else the process
    /// environment's.
    pub fn value(&self, key: &str) -> Option<&str> {
        let value = match self.file_values.get(key) {
            Some(assigned) => assigned.value.as_str(),
            None => self.process_values.get(key)?.as_str(),
        };
        (!value.is_empty()).then_some(value)
    }

    /// [`Self::value`], or `default` when the key is unset.
    pub fn value_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.value(key).unwrap_or(default)
    }

    /// [`Self::value`], or [`SettingError::Missing`] when the key is unset.
    pub fn required(&self, key: &str) -> Result<&str, SettingError> {
        self.value(key).ok_or_else(|| self.missing(key))
    }

    /// The error for a key the command needs and does not have.
    pub fn missing(&self, key: &str) -> SettingError {
        SettingError::Missing {
            key: key.to_string(),
            path: self.path.clone(),
        }
    }

    /// The error for a key whose value the command refuses, pointing at where the value came
    /// from. A key with no value at all is reported as [`SettingError::Missing`].
    pub fn invalid(&self, key: &str, problem: impl Into<String>) -> SettingError {
        let origin = match self.file_values.get(key) {
            Some(assigned) if !assigned.value.is_empty() => SettingOrigin::File {
                path: self.path.clone(),
                line: assigned.line,
            },
            None if self.value(key).is_some() => SettingOrigin::ProcessEnvironment,
            _ => return self.missing(key),
        };
        SettingError::Invalid {
            key: key.to_string(),
            origin,
            problem: problem.into(),
        }
    }

    /// The error for an unset key whose default cannot be derived from here.
    pub fn not_derivable(
        &self,
        key: &str,
        reason: impl Into<String>,
        remedy: impl Into<String>,
    ) -> SettingError {
        SettingError::NotDerivable {
            key: key.to_string(),
            path: self.path.clone(),
            reason: reason.into(),
            remedy: remedy.into(),
        }
    }

    /// The deploy host, parsed from [`DEPLOY_HOST_KEY`].
    pub fn deploy_host(&self) -> Result<DeployHost, SettingError> {
        let raw = self.required(DEPLOY_HOST_KEY)?;
        DeployHost::parse(raw).map_err(|problem| self.invalid(DEPLOY_HOST_KEY, problem))
    }

    /// How `ssh` reaches the deploy host: [`SshBase::from_settings`] over [`SSH_PASSWORD_KEY`]
    /// (first) and [`SSH_IDENTITY_FILE_KEY`].
    pub fn ssh_base(&self) -> SshBase {
        SshBase::from_settings(
            self.value(SSH_PASSWORD_KEY),
            self.value(SSH_IDENTITY_FILE_KEY),
        )
    }
}

/// The process environment now, keeping the variables whose name and value are both UTF-8.
fn process_snapshot() -> Vec<(String, String)> {
    std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}

#[cfg(test)]
#[path = "tests/deploy_environment_tests.rs"]
mod tests;
