//! Why a configuration is refused. Every error names the key or file at fault; none quotes a
//! secret.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("cannot read the configuration file {path}: {source}")]
    FileUnreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("the configuration file {path} is invalid: {message}")]
    FileMalformed { path: PathBuf, message: String },
    #[error("api_base_url {value:?} {problem}")]
    ApiBaseUrlInvalid {
        value: String,
        problem: &'static str,
    },
    #[error("poll_interval_seconds must be 1 to 300, not {0}")]
    PollIntervalOutOfRange(u64),
    #[error("game_server.systemd_user_unit {value:?} {problem}")]
    UnitNameInvalid {
        value: String,
        problem: &'static str,
    },
    #[error("game_server.start_dwell_seconds must be 0 to 30, not {0}")]
    StartDwellOutOfRange(u64),
    #[error("game_server.systemctl_program must be an absolute path, not {0:?}")]
    SystemctlProgramNotAbsolute(PathBuf),
    #[error("game_server.server_config_path {path:?} {problem}")]
    ServerConfigFileRejected {
        path: PathBuf,
        problem: ServerConfigFileProblem,
    },
    #[error("rcon.address {0:?} is not an IP address")]
    RconAddressInvalid(String),
    #[error("rcon.port must be 1 to 65535")]
    RconPortInvalid,
    #[error("{key} {path:?} {problem}")]
    SecretFileRejected {
        key: &'static str,
        path: PathBuf,
        problem: SecretFileProblem,
    },
}

/// What is wrong with the dedicated server's config file.
#[derive(Debug, Error)]
pub enum ServerConfigFileProblem {
    #[error("must be an absolute path")]
    RelativePath,
    #[error("cannot be inspected: {0}")]
    Unreadable(std::io::Error),
    #[error("is not a regular file")]
    NotRegularFile,
    #[error("is not writable by the agent: {0}")]
    NotWritable(std::io::Error),
}

/// What is wrong with a secret file.
#[derive(Debug, Error)]
pub enum SecretFileProblem {
    #[error("must be an absolute path")]
    RelativePath,
    #[error("cannot be read: {0}")]
    Unreadable(std::io::Error),
    #[error("is not a regular file")]
    NotRegularFile,
    #[error(
        "is accessible to other users (mode {mode:04o}); restrict it to its owner, for example \
         with chmod 600"
    )]
    AccessibleToOthers { mode: u32 },
    #[error("is larger than {limit} bytes")]
    TooLarge { limit: u64 },
    #[error("does not hold a valid secret: {0}")]
    InvalidContent(&'static str),
}
