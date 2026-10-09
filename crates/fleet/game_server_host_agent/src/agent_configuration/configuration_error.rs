//! Why a configuration is refused. Every error names the key or file at fault; none quotes a
//! secret.

use std::path::PathBuf;

use thiserror::Error;

use crate::process_control::SystemdUnitNameProblem;

/// A configuration the agent refuses to start with; the message names the key or file at fault.
#[derive(Debug, Error)]
pub enum ConfigurationError {
    /// The configuration file cannot be read.
    #[error("cannot read the configuration file {path}: {source}")]
    FileUnreadable {
        /// The configuration file.
        path: PathBuf,
        /// The read failure.
        source: std::io::Error,
    },
    /// The configuration file is not TOML of the configuration file's shape.
    #[error("the configuration file {path} is invalid: {message}")]
    FileMalformed {
        /// The configuration file.
        path: PathBuf,
        /// The TOML parser's description of the fault.
        message: String,
    },
    /// `api_base_url` is not an https origin (or a plain http loopback origin) without
    /// credentials, query or fragment.
    #[error("api_base_url {value:?} {problem}")]
    ApiBaseUrlInvalid {
        /// The configured value.
        value: String,
        /// What is wrong with it, phrased to follow the value.
        problem: &'static str,
    },
    /// `poll_interval_seconds` lies outside 1 to 300; carries the configured value.
    #[error("poll_interval_seconds must be 1 to 300, not {0}")]
    PollIntervalOutOfRange(u64),
    /// `game_server.systemd_user_unit` is not a valid systemd service unit name.
    #[error("game_server.systemd_user_unit {value:?} {problem}")]
    UnitNameInvalid {
        /// The configured value.
        value: String,
        /// What is wrong with it, phrased to follow the value.
        problem: SystemdUnitNameProblem,
    },
    /// `game_server.start_dwell_seconds` lies outside 0 to 30; carries the configured value.
    #[error("game_server.start_dwell_seconds must be 0 to 30, not {0}")]
    StartDwellOutOfRange(u64),
    /// `game_server.systemctl_program` is a relative path; carries the configured path.
    #[error("game_server.systemctl_program must be an absolute path, not {0:?}")]
    SystemctlProgramNotAbsolute(PathBuf),
    /// `game_server.server_config_path` does not name a config file the agent can rewrite.
    #[error("game_server.server_config_path {path:?} {problem}")]
    ServerConfigFileRejected {
        /// The configured path.
        path: PathBuf,
        /// What is wrong with the file.
        problem: ServerConfigFileProblem,
    },
    /// `rcon.address` is not an IP address; carries the configured value.
    #[error("rcon.address {0:?} is not an IP address")]
    RconAddressInvalid(String),
    /// `rcon.port` is 0.
    #[error("rcon.port must be 1 to 65535")]
    RconPortInvalid,
    /// A secret file key (`credential_file`, `rcon.password_file`) names a file that cannot
    /// hold the secret safely.
    #[error("{key} {path:?} {problem}")]
    SecretFileRejected {
        /// The configuration key that names the file.
        key: &'static str,
        /// The configured path.
        path: PathBuf,
        /// What is wrong with the file.
        problem: SecretFileProblem,
    },
}

/// What is wrong with the dedicated server's config file.
#[derive(Debug, Error)]
pub enum ServerConfigFileProblem {
    /// The path is relative.
    #[error("must be an absolute path")]
    RelativePath,
    /// The file's metadata cannot be read; carries the failure.
    #[error("cannot be inspected: {0}")]
    Unreadable(std::io::Error),
    /// The path names a directory, a device or another non-regular file.
    #[error("is not a regular file")]
    NotRegularFile,
    /// The agent cannot open the file for writing; carries the failure.
    #[error("is not writable by the agent: {0}")]
    NotWritable(std::io::Error),
}

/// What is wrong with a secret file.
#[derive(Debug, Error)]
pub enum SecretFileProblem {
    /// The path is relative.
    #[error("must be an absolute path")]
    RelativePath,
    /// The file or its metadata cannot be read; carries the failure.
    #[error("cannot be read: {0}")]
    Unreadable(std::io::Error),
    /// The path names a directory, a device or another non-regular file.
    #[error("is not a regular file")]
    NotRegularFile,
    /// A group or other permission bit is set.
    #[error(
        "is accessible to other users (mode {mode:04o}); restrict it to its owner, for example \
         with chmod 600"
    )]
    AccessibleToOthers {
        /// The file's permission bits (`mode & 0o777`).
        mode: u32,
    },
    /// The file is larger than a secret may be.
    #[error("is larger than {limit} bytes")]
    TooLarge {
        /// The largest size accepted, in bytes.
        limit: u64,
    },
    /// The content is not UTF-8 text or fails the secret's format check; carries the reason.
    #[error("does not hold a valid secret: {0}")]
    InvalidContent(&'static str),
}
