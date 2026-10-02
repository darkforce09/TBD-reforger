//! The deploy settings file, `deploy.env`, read the same way by every command that reaches a host.
//!
//! **Role:** loads `deploy.env` and answers each setting under one precedence rule: the file decides
//! every key it assigns, an empty assignment counting as unset, and the process environment fills
//! only the keys the file never assigns. An explicit command-line flag, where a command has one,
//! beats both; that choice stays with the command.
//!
//! **Position:** shared plumbing under the `deploy`, `setup`, `debug` and `mod` command groups.
//! [`deploy_environment_path`] picks the file (the `DEPLOY_ENV` variable, else
//! [`crate::core::repository_layout::DEPLOY_ENV`] in the checkout); `assignment_syntax` owns the
//! file's grammar, [`DeployHost`] the value of `TBD_SSH_HOST`, and [`DeployHostFolder`] the remote
//! folders that default under the deploy user's home.
//!
//! **Signals & state:** a [`DeployEnvironment`] is an immutable snapshot of the file and of the
//! process environment, taken when it is loaded; nothing here writes either.
//!
//! **Invariants:** the file is parsed and never executed; a stale exported variable never
//! overrides a key the file assigns; a present but unusable value is reported as `<path>:<line>`,
//! and a missing one names the file.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::repository_layout;

mod assignment_syntax;
mod deploy_host;
mod deploy_host_folders;

pub use deploy_host::{DeployHost, first_ipv4_address};
pub use deploy_host_folders::DeployHostFolder;

/// The process variable that points every command at another deploy settings file.
pub const DEPLOY_ENV_OVERRIDE_VARIABLE: &str = "DEPLOY_ENV";

/// The setting that names the deploy host, as `user@host` or `host`.
pub const DEPLOY_HOST_KEY: &str = "TBD_SSH_HOST";

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

/// Why the deploy settings file could not be loaded.
#[derive(Debug)]
pub enum DeployEnvironmentError {
    /// No file at the path, for a command that needs one.
    Missing { path: PathBuf },
    /// The file exists and reading it failed.
    Unreadable { path: PathBuf, error: io::Error },
    /// A line breaks the `KEY=VALUE` grammar.
    Syntax {
        path: PathBuf,
        line: usize,
        message: String,
    },
}

impl fmt::Display for DeployEnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { path } => write!(
                f,
                "Missing {} — copy from {}",
                path.display(),
                repository_layout::DEPLOY_ENV_EXAMPLE
            ),
            Self::Unreadable { path, error } => {
                write!(f, "could not read {}: {error}", path.display())
            }
            Self::Syntax {
                path,
                line,
                message,
            } => write!(f, "{}:{line}: {message}", path.display()),
        }
    }
}

impl std::error::Error for DeployEnvironmentError {}

/// Where a setting's value came from, so an error points at the edit to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingOrigin {
    /// Assigned in the deploy settings file, on this line.
    File { path: PathBuf, line: usize },
    /// Taken from the process environment, because the file never assigns the key.
    ProcessEnvironment,
}

/// A setting a command cannot use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingError {
    /// The key is unset, or assigned empty, and the command needs it.
    Missing { key: String, path: PathBuf },
    /// The key holds a value the command refuses.
    Invalid {
        key: String,
        origin: SettingOrigin,
        problem: String,
    },
    /// The key is unset and the value it defaults to cannot be derived from here.
    NotDerivable {
        key: String,
        path: PathBuf,
        reason: String,
        remedy: String,
    },
}

impl fmt::Display for SettingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { key, path } => {
                write!(f, "{key} is not set: add it to {}", path.display())
            }
            Self::Invalid {
                key,
                origin: SettingOrigin::File { path, line },
                problem,
            } => write!(f, "{}:{line}: {key}: {problem}", path.display()),
            Self::Invalid {
                key,
                origin: SettingOrigin::ProcessEnvironment,
                problem,
            } => write!(f, "{key} (process environment): {problem}"),
            Self::NotDerivable {
                key,
                path,
                reason,
                remedy,
            } => write!(
                f,
                "{key} is unset and {reason}: {remedy} or set {key} in {}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SettingError {}

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
    pub fn load_required(path: &Path) -> Result<Self, DeployEnvironmentError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_text(path, Some(&text), process_snapshot()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Err(DeployEnvironmentError::Missing {
                    path: path.to_path_buf(),
                })
            }
            Err(error) => Err(DeployEnvironmentError::Unreadable {
                path: path.to_path_buf(),
                error,
            }),
        }
    }

    /// Loads the file when it exists; without it every value comes from the process environment.
    pub fn load_if_present(path: &Path) -> Result<Self, DeployEnvironmentError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(DeployEnvironmentError::Unreadable {
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
    ) -> Result<Self, DeployEnvironmentError> {
        let assignments = match file_text {
            Some(text) => {
                assignment_syntax::parse_assignments(text).map_err(|(line, message)| {
                    DeployEnvironmentError::Syntax {
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
}

/// The process environment now, keeping the variables whose name and value are both UTF-8.
fn process_snapshot() -> Vec<(String, String)> {
    std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}

#[cfg(test)]
#[path = "tests/deploy_environment/tests.rs"]
mod tests;
